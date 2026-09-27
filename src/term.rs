//! A local shell. The bytes stay on this computer.

use crate::theme::{self, CREAM, PANEL};
use eframe::egui::{self, Color32, FontId, Rect, Vec2};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use vte::{Params, Perform};

const SCROLL_MAX: usize = 400;

#[derive(Clone, Copy)]
struct Cell {
    ch: char,
    fg: u8,
    bg: u8,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: 0,
            bg: 0,
        }
    }
}

pub struct Grid {
    cols: usize,
    rows: usize,
    cells: Vec<Cell>,
    cx: usize,
    cy: usize,
    fg: u8,
    bg: u8,
    bold: bool,
    color_base: u8,
    saved: Option<(usize, usize)>,
    scroll: Vec<Vec<Cell>>,
}

impl Grid {
    pub fn new(cols: usize, rows: usize) -> Self {
        let cols = cols.max(8);
        let rows = rows.max(4);
        Self {
            cols,
            rows,
            cells: vec![Cell::default(); cols * rows],
            cx: 0,
            cy: 0,
            fg: 0,
            bg: 0,
            bold: false,
            color_base: 255,
            saved: None,
            scroll: Vec::new(),
        }
    }

    pub fn scroll_len(&self) -> usize {
        self.scroll.len()
    }

    pub fn refit(&mut self, cols: usize, rows: usize) {
        let cols = cols.max(8);
        let rows = rows.max(4);
        if cols == self.cols && rows == self.rows {
            return;
        }
        let mut cursor_at = self.scroll.len() + self.cy;
        let mut lines = std::mem::take(&mut self.scroll);
        for y in 0..self.rows {
            let mut row = Vec::with_capacity(cols);
            for x in 0..cols {
                row.push(if x < self.cols {
                    self.cell(x, y)
                } else {
                    Cell::default()
                });
            }
            lines.push(row);
        }
        for line in &mut lines {
            line.resize(cols, Cell::default());
        }
        while lines.len() > cursor_at + 1 && lines.last().is_some_and(|line| line.iter().all(|c| c.ch == ' '))
        {
            lines.pop();
        }
        if lines.is_empty() {
            lines.push(vec![Cell::default(); cols]);
            cursor_at = 0;
        }
        cursor_at = cursor_at.min(lines.len() - 1);
        while lines.len() > SCROLL_MAX + rows {
            lines.remove(0);
            cursor_at = cursor_at.saturating_sub(1);
        }
        let below = lines.len().saturating_sub(cursor_at + 1).min(rows - 1);
        let above = rows - 1 - below;
        let mut start = cursor_at.saturating_sub(above);
        if lines.len() <= rows {
            start = 0;
        } else if start + rows > lines.len() {
            start = lines.len() - rows;
        }
        if cursor_at < start {
            start = cursor_at;
        }
        self.scroll = lines.drain(..start).collect();
        self.cols = cols;
        self.rows = rows;
        self.cells = vec![Cell::default(); cols * rows];
        for (y, row) in lines.into_iter().take(rows).enumerate() {
            for (x, cell) in row.into_iter().take(cols).enumerate() {
                self.cells[y * cols + x] = cell;
            }
        }
        self.cy = cursor_at.saturating_sub(start).min(rows - 1);
        self.cx = self.cx.min(cols.saturating_sub(1));
    }

    fn copy_row(&self, index: usize, out: &mut Vec<Cell>) {
        out.clear();
        if index < self.scroll.len() {
            out.extend_from_slice(&self.scroll[index]);
            out.resize(self.cols, Cell::default());
            return;
        }
        let y = index - self.scroll.len();
        if y >= self.rows {
            out.resize(self.cols, Cell::default());
            return;
        }
        let start = y * self.cols;
        out.extend_from_slice(&self.cells[start..start + self.cols]);
    }

    pub fn cursor(&self) -> (usize, usize) {
        (self.cx, self.cy)
    }

    fn cell(&self, x: usize, y: usize) -> Cell {
        self.cells
            .get(y * self.cols + x)
            .copied()
            .unwrap_or_default()
    }

    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.cols + x
    }

    fn put(&mut self, ch: char) {
        if self.cx >= self.cols {
            self.cx = 0;
            self.linefeed();
        }
        if self.cy >= self.rows {
            self.linefeed();
        }
        let i = self.idx(self.cx.min(self.cols - 1), self.cy.min(self.rows - 1));
        self.cells[i] = Cell {
            ch,
            fg: self.fg,
            bg: self.bg,
        };
        self.cx += 1;
    }

    fn linefeed(&mut self) {
        self.cx = 0;
        if self.cy + 1 >= self.rows {
            let row: Vec<Cell> = self.cells[..self.cols].to_vec();
            self.scroll.push(row);
            if self.scroll.len() > SCROLL_MAX {
                self.scroll.remove(0);
            }
            self.cells.copy_within(self.cols.., 0);
            let start = self.cols * (self.rows - 1);
            for c in &mut self.cells[start..] {
                *c = Cell::default();
            }
            self.cy = self.rows - 1;
        } else {
            self.cy += 1;
        }
    }

    fn cr(&mut self) {
        self.cx = 0;
    }

    fn backspace(&mut self) {
        if self.cx > 0 {
            self.cx -= 1;
        }
    }

    fn tab(&mut self) {
        let next = (self.cx / 8 + 1) * 8;
        self.cx = next.min(self.cols.saturating_sub(1));
    }

    fn reset_sgr(&mut self) {
        self.fg = 0;
        self.bg = 0;
        self.bold = false;
        self.color_base = 255;
    }

    fn apply_fg(&mut self) {
        if self.color_base == 255 {
            self.fg = 0;
        } else if self.bold {
            self.fg = self.color_base + 9;
        } else {
            self.fg = self.color_base + 1;
        }
    }

    fn erase_to(&mut self, to_cursor: bool) {
        let y = self.cy.min(self.rows - 1);
        let x = self.cx.min(self.cols - 1);
        if to_cursor {
            for row in 0..y {
                for col in 0..self.cols {
                    self.cells[row * self.cols + col] = Cell::default();
                }
            }
            for col in 0..=x {
                self.cells[y * self.cols + col] = Cell::default();
            }
        } else {
            for col in x..self.cols {
                self.cells[y * self.cols + col] = Cell::default();
            }
            for row in (y + 1)..self.rows {
                for col in 0..self.cols {
                    self.cells[row * self.cols + col] = Cell::default();
                }
            }
        }
    }

    fn clear_all(&mut self) {
        self.cells.fill(Cell::default());
        self.cx = 0;
        self.cy = 0;
    }

    fn erase_line(&mut self, mode: u16) {
        let y = self.cy.min(self.rows - 1);
        let (a, b) = match mode {
            1 => (0, self.cx.min(self.cols - 1) + 1),
            2 => (0, self.cols),
            _ => (self.cx.min(self.cols - 1), self.cols),
        };
        let cols = self.cols;
        for x in a..b {
            let i = y * cols + x;
            self.cells[i] = Cell::default();
        }
    }

    fn move_to(&mut self, x: usize, y: usize) {
        self.cx = x.min(self.cols.saturating_sub(1));
        self.cy = y.min(self.rows.saturating_sub(1));
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        let mut parser = vte::Parser::new();
        for b in bytes {
            parser.advance(self, *b);
        }
    }
}

fn param(params: &Params, i: usize) -> u16 {
    params
        .iter()
        .nth(i)
        .and_then(|p| p.first().copied())
        .unwrap_or(0)
}

impl Perform for Grid {
    fn print(&mut self, c: char) {
        self.put(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            0x08 => self.backspace(),
            0x09 => self.tab(),
            0x0a | 0x0b | 0x0c => self.linefeed(),
            0x0d => self.cr(),
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, action: char) {
        let p0 = param(params, 0);
        match action {
            'A' => self.move_to(self.cx, self.cy.saturating_sub(p0.max(1) as usize)),
            'B' => self.move_to(self.cx, self.cy + p0.max(1) as usize),
            'C' => self.move_to(self.cx + p0.max(1) as usize, self.cy),
            'D' => self.move_to(self.cx.saturating_sub(p0.max(1) as usize), self.cy),
            'G' => self.move_to(p0.saturating_sub(1) as usize, self.cy),
            'H' | 'f' => {
                let row = p0.max(1) as usize - 1;
                let col = param(params, 1).max(1) as usize - 1;
                self.move_to(col, row);
            }
            'J' => match p0 {
                2 => self.clear_all(),
                1 => self.erase_to(true),
                _ => self.erase_to(false),
            },
            'K' => self.erase_line(p0),
            'm' => {
                if params.iter().next().is_none() {
                    self.reset_sgr();
                }
                for group in params.iter() {
                    let n = group.first().copied().unwrap_or(0);
                    match n {
                        0 => self.reset_sgr(),
                        1 => {
                            self.bold = true;
                            self.apply_fg();
                        }
                        22 => {
                            self.bold = false;
                            self.apply_fg();
                        }
                        30..=37 => {
                            self.color_base = (n - 30) as u8;
                            self.apply_fg();
                        }
                        39 => {
                            self.color_base = 255;
                            self.fg = 0;
                        }
                        40..=47 => self.bg = (n - 40) as u8 + 1,
                        49 => self.bg = 0,
                        90..=97 => {
                            self.color_base = (n - 90) as u8;
                            self.fg = self.color_base + 9;
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, byte: u8) {
        match byte {
            b'7' => self.saved = Some((self.cx, self.cy)),
            b'8' => {
                if let Some((x, y)) = self.saved {
                    self.move_to(x, y);
                }
            }
            _ => {}
        }
    }
}

fn row_hash(line: &[Cell]) -> u64 {
    let mut h = 1469598103934665603u64;
    for cell in line {
        h ^= cell.ch as u64;
        h = h.wrapping_mul(1099511628211);
        h ^= cell.fg as u64;
        h = h.wrapping_mul(1099511628211);
        h ^= cell.bg as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

fn row_job(line: &[Cell], font: FontId) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let mut i = 0;
    while i < line.len() {
        let cell = line[i];
        let mut end = i + 1;
        while end < line.len() && line[end].fg == cell.fg {
            end += 1;
        }
        let text: String = line[i..end].iter().map(|c| c.ch).collect();
        job.append(
            &text,
            0.0,
            egui::TextFormat {
                font_id: font.clone(),
                color: ansi(cell.fg, true),
                ..Default::default()
            },
        );
        i = end;
    }
    job
}

fn ansi(n: u8, default_fg: bool) -> Color32 {
    match n {
        1 => Color32::from_rgb(20, 20, 20),
        2 => Color32::from_rgb(220, 60, 60),
        3 => Color32::from_rgb(40, 160, 80),
        4 => Color32::from_rgb(200, 160, 40),
        5 => Color32::from_rgb(60, 110, 220),
        6 => Color32::from_rgb(180, 60, 180),
        7 => Color32::from_rgb(40, 170, 180),
        8 => Color32::from_rgb(220, 220, 220),
        9 => Color32::from_rgb(80, 80, 80),
        10 => Color32::from_rgb(255, 90, 90),
        11 => Color32::from_rgb(80, 230, 120),
        12 => Color32::from_rgb(255, 220, 80),
        13 => Color32::from_rgb(120, 160, 255),
        14 => Color32::from_rgb(255, 120, 220),
        15 => Color32::from_rgb(80, 240, 240),
        16 => Color32::from_rgb(255, 255, 255),
        _ if default_fg => CREAM,
        _ => PANEL,
    }
}

pub struct Shell {
    writer: Option<Box<dyn Write + Send>>,
    child: Option<Box<dyn portable_pty::Child + Send + Sync>>,
    master: Option<Box<dyn MasterPty + Send>>,
    rx: Receiver<Vec<u8>>,
    _tx_keep: Sender<Vec<u8>>,
    parser: vte::Parser,
    grid: Grid,
    cols: u16,
    rows: u16,
    view: usize,
    row_cache: Vec<Option<(u64, std::sync::Arc<egui::Galley>)>>,
    line_buf: Vec<Cell>,
    font_q: f32,
    pub err: String,
}

impl Shell {
    pub fn spawn(cols: u16, rows: u16) -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        Self::spawn_in(cols, rows, &cwd)
    }

    pub fn spawn_in(cols: u16, rows: u16, cwd: &std::path::Path) -> Self {
        let cols = cols.clamp(20, 240);
        let rows = rows.clamp(6, 80);
        let (tx, rx) = channel();
        let mut shell = Self {
            writer: None,
            child: None,
            master: None,
            rx,
            _tx_keep: tx.clone(),
            parser: vte::Parser::new(),
            grid: Grid::new(cols as usize, rows as usize),
            cols,
            rows,
            view: 0,
            row_cache: Vec::new(),
            line_buf: Vec::new(),
            font_q: 0.0,
            err: String::new(),
        };
        if let Err(e) = shell.open(cols, rows, tx, cwd) {
            shell.err = e;
        }
        shell
    }

    fn open(&mut self, cols: u16, rows: u16, tx: Sender<Vec<u8>>, cwd: &std::path::Path) -> Result<(), String> {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("The terminal did not open. {e}"))?;
        let mut cmd = CommandBuilder::new(shell_program());
        cmd.cwd(cwd);
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("The shell did not start. {e}"))?;
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| e.to_string())?;
        let writer = pair.master.take_writer().map_err(|e| e.to_string())?;
        thread::spawn(move || {
            let mut reader = reader;
            let mut buf = [0u8; 65536];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        self.writer = Some(writer);
        self.child = Some(child);
        self.master = Some(pair.master);
        Ok(())
    }

    pub fn poll(&mut self) {
        let mut n = 0usize;
        while let Ok(buf) = self.rx.try_recv() {
            for b in &buf {
                self.parser.advance(&mut self.grid, *b);
            }
            n += buf.len();
            if n > 2_000_000 {
                break;
            }
        }
    }

    pub fn scroll_by(&mut self, lines: i32) {
        let max = self.grid.scroll_len();
        if lines > 0 {
            self.view = (self.view + lines as usize).min(max);
        } else {
            self.view = self.view.saturating_sub((-lines) as usize);
        }
    }

    pub fn write_str(&mut self, text: &str) {
        self.view = 0;
        if let Some(w) = self.writer.as_mut() {
            let _ = w.write_all(text.as_bytes());
            let _ = w.flush();
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        let cols = cols.clamp(20, 240);
        let rows = rows.clamp(6, 80);
        if cols == self.cols && rows == self.rows {
            return;
        }
        self.cols = cols;
        self.rows = rows;
        self.grid.refit(cols as usize, rows as usize);
        self.view = self.view.min(self.grid.scroll_len());
        self.row_cache.clear();
        if let Some(master) = self.master.as_ref() {
            let _ = master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            });
        }
    }

    pub fn paint(&mut self, ui: &egui::Ui, rect: Rect) {
        ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(8, 10, 14));
        let rows = self.rows as usize;
        let cols = self.cols as usize;
        let total = self.grid.scroll_len() + rows;
        let view = self.view.min(self.grid.scroll_len());
        let start = total.saturating_sub(rows + view);
        let cw = (rect.width() / cols as f32).max(7.0);
        let ch = (rect.height() / rows as f32).max(12.0);
        let font = mono_fit(ui, cw, ch);
        let q = (font.size * 4.0).round();
        if (self.font_q - q).abs() > 0.1 || self.row_cache.len() != rows {
            self.font_q = q;
            self.row_cache = vec![None; rows];
        }
        for y in 0..rows {
            self.grid.copy_row(start + y, &mut self.line_buf);
            let hash = row_hash(&self.line_buf);
            if self.row_cache[y].as_ref().map(|(h, _)| *h) != Some(hash) {
                let galley = ui
                    .ctx()
                    .fonts(|f| f.layout_job(row_job(&self.line_buf, font.clone())));
                self.row_cache[y] = Some((hash, galley));
            }
            let min = egui::pos2(rect.left(), rect.top() + y as f32 * ch);
            for (x, cell) in self.line_buf.iter().enumerate() {
                if cell.bg == 0 {
                    continue;
                }
                let run = Rect::from_min_size(
                    egui::pos2(rect.left() + x as f32 * cw, min.y),
                    Vec2::new(cw, ch),
                );
                ui.painter().rect_filled(run, 0.0, ansi(cell.bg, false));
            }
            if let Some((_, galley)) = &self.row_cache[y] {
                ui.painter()
                    .galley(min + Vec2::new(1.0, 1.0), galley.clone(), Color32::WHITE);
            }
        }
        if self.view == 0 {
            let cursor = Rect::from_min_size(
                egui::pos2(
                    rect.left() + self.grid.cx as f32 * cw,
                    rect.top() + self.grid.cy as f32 * ch,
                ),
                Vec2::new(2.0, ch),
            );
            ui.painter().rect_filled(cursor, 0.0, theme::ACID);
        }
    }
}

fn mono_fit(ui: &egui::Ui, cw: f32, ch: f32) -> FontId {
    let family = theme::mono();
    let mut size = (ch * 0.86).clamp(8.0, 32.0);
    for _ in 0..5 {
        let id = FontId::new(size, family.clone());
        let (gw, rh) = ui.ctx().fonts(|f| (f.glyph_width(&id, 'M'), f.row_height(&id)));
        if gw <= 0.5 || rh <= 0.5 {
            break;
        }
        let mut next = size * (cw / gw);
        if rh * (next / size) > ch {
            next = size * (ch / rh);
        }
        next = next.clamp(6.0, 40.0);
        if (next - size).abs() < 0.2 {
            size = next;
            break;
        }
        size = next;
    }
    FontId::new(size, family)
}

impl Drop for Shell {
    fn drop(&mut self) {
        self.writer.take();
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
        }
    }
}

pub fn shell_program() -> String {
    #[cfg(windows)]
    {
        if crate::sys::which("powershell").is_some() {
            return "powershell.exe".into();
        }
        return "cmd.exe".into();
    }
    #[cfg(not(windows))]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into())
    }
}

pub fn list_commands() -> Vec<String> {
    let mut names = BTreeSet::new();
    let Ok(path) = std::env::var("PATH") else {
        return Vec::new();
    };
    for dir in std::env::split_paths(&path) {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name.is_empty() {
                continue;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let Ok(meta) = entry.metadata() else {
                    continue;
                };
                if meta.permissions().mode() & 0o111 == 0 {
                    continue;
                }
            }
            #[cfg(windows)]
            {
                let lower = name.to_lowercase();
                if !lower.ends_with(".exe")
                    && !lower.ends_with(".bat")
                    && !lower.ends_with(".cmd")
                    && !lower.ends_with(".com")
                {
                    continue;
                }
            }
            names.insert(name);
        }
    }
    let mut names: Vec<String> = names.into_iter().collect();
    names.sort_by(|a, b| {
        a.to_lowercase()
            .cmp(&b.to_lowercase())
            .then_with(|| a.cmp(b))
    });
    names
}

pub fn handle_key(shell: &mut Shell, event: &egui::Event) {
    match event {
        egui::Event::Text(text) => shell.write_str(text),
        egui::Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } => {
            if modifiers.command && *key == egui::Key::C {
                shell.write_str("\u{3}");
                return;
            }
            let seq = match key {
                egui::Key::Enter => "\r",
                egui::Key::Backspace => "\u{7f}",
                egui::Key::Tab => "\t",
                egui::Key::Escape => "\u{1b}",
                egui::Key::ArrowUp => "\u{1b}[A",
                egui::Key::ArrowDown => "\u{1b}[B",
                egui::Key::ArrowRight => "\u{1b}[C",
                egui::Key::ArrowLeft => "\u{1b}[D",
                egui::Key::Home => "\u{1b}[H",
                egui::Key::End => "\u{1b}[F",
                egui::Key::PageUp => "\u{1b}[5~",
                egui::Key::PageDown => "\u{1b}[6~",
                egui::Key::Delete => "\u{1b}[3~",
                _ => "",
            };
            if !seq.is_empty() {
                shell.write_str(seq);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_prints_a_line() {
        let mut grid = Grid::new(20, 5);
        grid.feed(b"hi\r\n");
        assert_eq!(grid.cell(0, 0).ch, 'h');
        assert_eq!(grid.cell(1, 0).ch, 'i');
        assert_eq!(grid.cursor(), (0, 1));
        grid.feed(b"\x1b[1;32mG");
        assert!(grid.cell(0, 1).fg >= 9);
        grid.refit(10, 4);
        assert_eq!(grid.cell(0, 0).ch, 'h');
    }

    #[test]
    fn command_list_is_a_list() {
        let cmds = list_commands();
        assert!(cmds.windows(2).all(|w| {
            w[0].to_lowercase() < w[1].to_lowercase()
                || (w[0].to_lowercase() == w[1].to_lowercase() && w[0] <= w[1])
        }));
    }

    #[test]
    fn shell_starts_in_the_given_folder() {
        let dir = std::env::temp_dir().join(format!("bn-term-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let mut shell = Shell::spawn_in(60, 16, &dir);
        if !shell.err.is_empty() {
            let _ = std::fs::remove_dir_all(&dir);
            return;
        }
        shell.write_str("pwd\r");
        let needle = dir.display().to_string();
        let start = std::time::Instant::now();
        let mut saw = false;
        while start.elapsed() < std::time::Duration::from_secs(4) {
            shell.poll();
            let mut word = String::new();
            for y in 0..16 {
                for x in 0..60 {
                    word.push(shell.grid.cell(x, y).ch);
                }
            }
            if word.contains(&needle) {
                saw = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        let _ = std::fs::remove_dir_all(&dir);
        assert!(saw, "shell did not start in the folder");
    }

    #[test]
    fn shell_echoes_a_line() {
        let mut shell = Shell::spawn(48, 16);
        if !shell.err.is_empty() {
            return;
        }
        shell.write_str("printf blight-term-ok\r");
        let start = std::time::Instant::now();
        let mut saw = false;
        while start.elapsed() < std::time::Duration::from_secs(4) {
            shell.poll();
            let mut word = String::new();
            for y in 0..16 {
                for x in 0..48 {
                    word.push(shell.grid.cell(x, y).ch);
                }
            }
            if word.contains("blight-term-ok") {
                saw = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        assert!(saw, "shell did not print");
    }
}

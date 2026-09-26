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
            scroll: Vec::new(),
        }
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
            'J' if p0 == 2 => self.clear_all(),
            'K' => self.erase_line(p0),
            'm' => {
                if params.iter().next().is_none() {
                    self.fg = 0;
                    self.bg = 0;
                }
                for group in params.iter() {
                    let n = group.first().copied().unwrap_or(0);
                    match n {
                        0 => {
                            self.fg = 0;
                            self.bg = 0;
                        }
                        30..=37 => self.fg = (n - 30) as u8 + 1,
                        39 => self.fg = 0,
                        40..=47 => self.bg = (n - 40) as u8 + 1,
                        49 => self.bg = 0,
                        90..=97 => self.fg = (n - 90) as u8 + 1,
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
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
    pub err: String,
}

impl Shell {
    pub fn spawn(cols: u16, rows: u16) -> Self {
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
            err: String::new(),
        };
        if let Err(e) = shell.open(cols, rows, tx) {
            shell.err = e;
        }
        shell
    }

    fn open(&mut self, cols: u16, rows: u16, tx: Sender<Vec<u8>>) -> Result<(), String> {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("The terminal did not open. {e}"))?;
        let mut cmd = CommandBuilder::new(shell_program());
        cmd.cwd(std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")));
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
            let mut buf = [0u8; 4096];
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
            if n > 65_536 {
                break;
            }
        }
    }

    pub fn write_str(&mut self, text: &str) {
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
        self.grid = Grid::new(cols as usize, rows as usize);
        if let Some(master) = self.master.as_ref() {
            let _ = master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            });
        }
    }

    pub fn paint(&self, ui: &egui::Ui, rect: Rect) {
        ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(8, 10, 14));
        let cw = (rect.width() / self.cols as f32).max(7.0);
        let ch = (rect.height() / self.rows as f32).max(12.0);
        let font = FontId::new(ch * 0.72, theme::mono());
        let mut y = 0usize;
        while y < self.rows as usize {
            let mut x = 0usize;
            while x < self.cols as usize {
                let cell = self.grid.cell(x, y);
                let mut end = x + 1;
                if cell.ch != ' ' || cell.bg != 0 {
                    while end < self.cols as usize {
                        let n = self.grid.cell(end, y);
                        if n.fg != cell.fg || n.bg != cell.bg || (n.ch == ' ' && n.bg == 0) {
                            break;
                        }
                        end += 1;
                    }
                    let min = egui::pos2(rect.left() + x as f32 * cw, rect.top() + y as f32 * ch);
                    let run = Rect::from_min_size(min, Vec2::new((end - x) as f32 * cw, ch));
                    if cell.bg != 0 {
                        ui.painter().rect_filled(run, 0.0, ansi(cell.bg, false));
                    }
                    let text: String = (x..end).map(|i| self.grid.cell(i, y).ch).collect();
                    ui.painter().text(
                        min + Vec2::new(1.0, 1.0),
                        egui::Align2::LEFT_TOP,
                        text,
                        font.clone(),
                        ansi(cell.fg, true),
                    );
                }
                x = end;
            }
            y += 1;
        }
        let cursor = Rect::from_min_size(
            egui::pos2(
                rect.left() + self.grid.cx as f32 * cw,
                rect.top() + self.grid.cy as f32 * ch,
            ),
            Vec2::new(cw.max(2.0), 2.0),
        );
        ui.painter().rect_filled(cursor, 0.0, theme::ACID);
    }
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

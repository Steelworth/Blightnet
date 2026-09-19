use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn hide(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let _ = cmd;
}

pub fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let p = dir.join(name);
        if p.is_file() {
            return Some(p);
        }
        #[cfg(windows)]
        {
            let exe = dir.join(format!("{name}.exe"));
            if exe.is_file() {
                return Some(exe);
            }
        }
    }
    None
}

pub fn ffmpeg_bin() -> Option<PathBuf> {
    if let Some(p) = which("ffmpeg") {
        return Some(p);
    }
    #[cfg(windows)]
    {
        for p in [
            PathBuf::from(r"C:\ffmpeg\bin\ffmpeg.exe"),
            PathBuf::from(r"C:\Program Files\ffmpeg\bin\ffmpeg.exe"),
            PathBuf::from(r"C:\Program Files (x86)\ffmpeg\bin\ffmpeg.exe"),
        ] {
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

pub fn open_path(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }
    #[cfg(windows)]
    {
        let mut c = Command::new("cmd");
        hide(&mut c);
        return c
            .args(["/C", "start", "", &path.to_string_lossy()])
            .spawn()
            .is_ok();
    }
    #[cfg(target_os = "macos")]
    {
        return Command::new("open").arg(path).spawn().is_ok();
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .or_else(|_| Command::new("gio").args(["open"]).arg(path).spawn())
            .or_else(|_| Command::new("mpv").arg(path).spawn())
            .or_else(|_| {
                Command::new("ffplay")
                    .args(["-autoexit", "-loglevel", "quiet"])
                    .arg(path)
                    .spawn()
            })
            .is_ok()
    }
}

pub fn download(url: &str, dest: &Path) -> bool {
    if let Some(parent) = dest.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    for bin in ["curl", "curl.exe"] {
        let mut c = Command::new(bin);
        hide(&mut c);
        if let Ok(st) = c.args(["-fsSL", "-o"]).arg(dest).arg(url).status() {
            if st.success() && dest.is_file() {
                return true;
            }
        }
    }
    #[cfg(windows)]
    {
        let mut c = Command::new("powershell");
        hide(&mut c);
        let uri = url.replace('\'', "''");
        let out = dest.to_string_lossy().replace('\'', "''");
        let script = format!(
            "try {{ Invoke-WebRequest -UseBasicParsing -Uri '{uri}' -OutFile '{out}' }} catch {{ exit 1 }}"
        );
        if let Ok(st) = c
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .status()
        {
            if st.success() && dest.is_file() {
                return true;
            }
        }
    }
    let _ = std::fs::remove_file(dest);
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn which_finds_nothing_bogus() {
        assert!(which("blightnet-no-such-binary-xyz").is_none());
    }
}

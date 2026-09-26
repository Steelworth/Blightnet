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

#[derive(Clone, Copy, Debug, Default)]
pub struct CpuTick {
    pub busy: u64,
    pub total: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct Machine {
    pub cpu: f32,
    pub gpu: Option<f32>,
    pub ram_used: u64,
    pub ram_total: u64,
    pub disk_free: u64,
    pub disk_total: u64,
}

impl Default for Machine {
    fn default() -> Self {
        Self {
            cpu: 0.0,
            gpu: None,
            ram_used: 0,
            ram_total: 0,
            disk_free: 0,
            disk_total: 0,
        }
    }
}

pub fn sample_machine(prev: &mut CpuTick, root: &Path) -> Machine {
    let (busy, total) = cpu_counters();
    let cpu = if prev.total > 0 && total > prev.total {
        let db = busy.saturating_sub(prev.busy) as f32;
        let dt = (total - prev.total) as f32;
        (db / dt * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };
    *prev = CpuTick { busy, total };
    let (ram_used, ram_total) = ram_bytes();
    let (disk_free, disk_total) = disk_bytes(root);
    poll_gpu();
    let gpu_raw = GPU_PCT.load(std::sync::atomic::Ordering::Relaxed);
    Machine {
        cpu,
        gpu: if gpu_raw < 0 { None } else { Some(gpu_raw as f32) },
        ram_used,
        ram_total,
        disk_free,
        disk_total,
    }
}

fn cpu_counters() -> (u64, u64) {
    #[cfg(target_os = "linux")]
    {
        if let Ok(text) = std::fs::read_to_string("/proc/stat") {
            if let Some(line) = text.lines().next() {
                let nums: Vec<u64> = line
                    .split_whitespace()
                    .skip(1)
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if nums.len() >= 4 {
                    let idle = nums[3] + nums.get(4).copied().unwrap_or(0);
                    let total: u64 = nums.iter().sum();
                    return (total.saturating_sub(idle), total);
                }
            }
        }
    }
    #[cfg(windows)]
    {
        if let Some((idle, total)) = win_times() {
            return (total.saturating_sub(idle), total);
        }
    }
    (0, 0)
}

fn ram_bytes() -> (u64, u64) {
    #[cfg(target_os = "linux")]
    {
        if let Ok(text) = std::fs::read_to_string("/proc/meminfo") {
            let mut total = 0u64;
            let mut avail = 0u64;
            for line in text.lines() {
                let mut parts = line.split_whitespace();
                let key = parts.next().unwrap_or("");
                let kb: u64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                if key.starts_with("MemTotal") {
                    total = kb.saturating_mul(1024);
                } else if key.starts_with("MemAvailable") {
                    avail = kb.saturating_mul(1024);
                }
            }
            if total > 0 {
                return (total.saturating_sub(avail), total);
            }
        }
    }
    #[cfg(windows)]
    {
        if let Some(pair) = win_ram() {
            return pair;
        }
    }
    (0, 0)
}

fn disk_bytes(root: &Path) -> (u64, u64) {
    #[cfg(unix)]
    {
        let mut path = root.to_path_buf();
        if !path.exists() {
            path = PathBuf::from(".");
        }
        let c = std::ffi::CString::new(path.to_string_lossy().as_bytes()).ok();
        if let Some(c) = c {
            let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
            if unsafe { libc::statvfs(c.as_ptr(), &mut st) } == 0 {
                let free = st.f_bavail as u64 * st.f_frsize as u64;
                let total = st.f_blocks as u64 * st.f_frsize as u64;
                return (free, total);
            }
        }
    }
    #[cfg(windows)]
    {
        if let Some(pair) = win_disk(root) {
            return pair;
        }
    }
    (0, 0)
}

static GPU_PCT: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);
static GPU_AT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn poll_gpu() {
    if let Some(v) = read_gpu_sysfs() {
        GPU_PCT.store(v, std::sync::atomic::Ordering::Relaxed);
        return;
    }
    if which("nvidia-smi").is_none() {
        return;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let last = GPU_AT.load(std::sync::atomic::Ordering::Relaxed);
    if now.saturating_sub(last) < 2 {
        return;
    }
    if GPU_AT
        .compare_exchange(last, now, std::sync::atomic::Ordering::Relaxed, std::sync::atomic::Ordering::Relaxed)
        .is_err()
    {
        return;
    }
    std::thread::spawn(|| {
        let mut cmd = Command::new("nvidia-smi");
        hide(&mut cmd);
        let Ok(out) = cmd
            .args(["--query-gpu=utilization.gpu", "--format=csv,noheader,nounits"])
            .output()
        else {
            return;
        };
        let text = String::from_utf8_lossy(&out.stdout);
        if let Some(n) = text
            .split(|c: char| !c.is_ascii_digit())
            .find(|s| !s.is_empty())
            .and_then(|s| s.parse::<i32>().ok())
        {
            GPU_PCT.store(n.clamp(0, 100), std::sync::atomic::Ordering::Relaxed);
        }
    });
}

fn read_gpu_sysfs() -> Option<i32> {
    let root = Path::new("/sys/class/drm");
    let rd = std::fs::read_dir(root).ok()?;
    for entry in rd.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("card") || name.contains('-') {
            continue;
        }
        let busy = entry.path().join("device/gpu_busy_percent");
        if let Ok(text) = std::fs::read_to_string(&busy) {
            if let Ok(n) = text.trim().parse::<i32>() {
                return Some(n.clamp(0, 100));
            }
        }
    }
    None
}

#[cfg(windows)]
fn filetime_u64(lo: u32, hi: u32) -> u64 {
    ((hi as u64) << 32) | lo as u64
}

#[cfg(windows)]
fn win_times() -> Option<(u64, u64)> {
    #[repr(C)]
    struct Ft {
        lo: u32,
        hi: u32,
    }
    extern "system" {
        fn GetSystemTimes(idle: *mut Ft, kernel: *mut Ft, user: *mut Ft) -> i32;
    }
    let mut idle = Ft { lo: 0, hi: 0 };
    let mut kernel = Ft { lo: 0, hi: 0 };
    let mut user = Ft { lo: 0, hi: 0 };
    if unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) } == 0 {
        return None;
    }
    let idle = filetime_u64(idle.lo, idle.hi);
    let kernel = filetime_u64(kernel.lo, kernel.hi);
    let user = filetime_u64(user.lo, user.hi);
    let total = kernel.saturating_add(user);
    Some((idle, total))
}

#[cfg(windows)]
fn win_ram() -> Option<(u64, u64)> {
    #[repr(C)]
    struct Mem {
        len: u32,
        load: u32,
        total: u64,
        avail: u64,
        _pad: [u64; 5],
    }
    extern "system" {
        fn GlobalMemoryStatusEx(lp: *mut Mem) -> i32;
    }
    let mut m = Mem {
        len: std::mem::size_of::<Mem>() as u32,
        load: 0,
        total: 0,
        avail: 0,
        _pad: [0; 5],
    };
    if unsafe { GlobalMemoryStatusEx(&mut m) } == 0 {
        return None;
    }
    Some((m.total.saturating_sub(m.avail), m.total))
}

#[cfg(windows)]
fn win_disk(root: &Path) -> Option<(u64, u64)> {
    use std::os::windows::ffi::OsStrExt;
    extern "system" {
        fn GetDiskFreeSpaceExW(dir: *const u16, free: *mut u64, total: *mut u64, total_free: *mut u64) -> i32;
    }
    let mut wide: Vec<u16> = root.as_os_str().encode_wide().collect();
    wide.push(0);
    let mut free = 0u64;
    let mut total = 0u64;
    if unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, &mut total, std::ptr::null_mut()) } == 0 {
        return None;
    }
    Some((free, total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn machine_sample_sees_memory() {
        let mut tick = CpuTick::default();
        let m = sample_machine(&mut tick, Path::new("."));
        assert!(m.ram_total > 0 || m.disk_total > 0);
        let again = sample_machine(&mut tick, Path::new("."));
        assert!(again.ram_total > 0 || again.disk_total > 0);
    }

    #[test]
    fn which_finds_nothing_bogus() {
        assert!(which("blightnet-no-such-binary-xyz").is_none());
    }
}

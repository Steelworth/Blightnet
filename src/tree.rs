//! One folder at a time. Nothing here is sent to the table.

use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub dir: bool,
    pub bytes: u64,
}

pub fn home_dir() -> Option<PathBuf> {
    let raw = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    let path = PathBuf::from(raw);
    path.is_dir().then_some(path)
}

pub fn computer_roots() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        let mut roots = Vec::new();
        for letter in b'A'..=b'Z' {
            let path = PathBuf::from(format!("{}:\\", letter as char));
            if path.is_dir() {
                roots.push(path);
            }
        }
        if roots.is_empty() {
            roots.push(PathBuf::from("C:\\"));
        }
        return roots;
    }
    #[cfg(not(windows))]
    {
        vec![PathBuf::from("/")]
    }
}

pub fn list_dir(path: &Path) -> Result<Vec<Entry>, String> {
    let rd = std::fs::read_dir(path).map_err(|_| "This folder did not open.".to_string())?;
    let mut rows = Vec::new();
    for ent in rd.flatten() {
        let path = ent.path();
        let name = ent.file_name().to_string_lossy().into_owned();
        if name.is_empty() {
            continue;
        }
        let meta = ent.metadata().ok();
        let dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let bytes = if dir { 0 } else { meta.map(|m| m.len()).unwrap_or(0) };
        rows.push(Entry {
            name,
            path,
            dir,
            bytes,
        });
    }
    rows.sort_by(|a, b| {
        b.dir
            .cmp(&a.dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(rows)
}

pub fn safe_name(name: &str) -> Option<&str> {
    let name = name.trim();
    if name.is_empty() || name == "." || name == ".." {
        return None;
    }
    if name.contains('/') || name.contains('\\') || name.contains('\0') {
        return None;
    }
    Some(name)
}

pub fn create_file(dir: &Path, name: &str) -> Result<PathBuf, String> {
    let name = safe_name(name).ok_or_else(|| "Use a name without a slash.".to_string())?;
    let path = dir.join(name);
    if path.exists() {
        return Err("That name is already there.".into());
    }
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|_| "The file was not created.".to_string())?;
    Ok(path)
}

pub fn create_dir(dir: &Path, name: &str) -> Result<PathBuf, String> {
    let name = safe_name(name).ok_or_else(|| "Use a name without a slash.".to_string())?;
    let path = dir.join(name);
    if path.exists() {
        return Err("That name is already there.".into());
    }
    std::fs::create_dir(&path).map_err(|_| "The folder was not created.".to_string())?;
    Ok(path)
}

pub fn is_fs_root(path: &Path) -> bool {
    if path == Path::new("/") {
        return true;
    }
    let text = path.to_string_lossy();
    let bytes = text.as_bytes();
    if bytes.len() == 2 && bytes[1] == b':' {
        return true;
    }
    if bytes.len() == 3 && bytes[1] == b':' && (bytes[2] == b'\\' || bytes[2] == b'/') {
        return true;
    }
    false
}

fn same_place(path: &Path, other: &Path) -> bool {
    match (path.canonicalize(), other.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => path == other,
    }
}

pub fn protected(path: &Path, app_root: &Path) -> bool {
    if is_fs_root(path) || same_place(path, app_root) {
        return true;
    }
    // A parent of this install would take the program with it.
    app_root.starts_with(path)
}

pub fn dir_has_child(path: &Path) -> bool {
    std::fs::read_dir(path)
        .map(|mut rd| rd.next().is_some())
        .unwrap_or(false)
}

pub fn remove_path(path: &Path, app_root: &Path) -> Result<(), String> {
    if protected(path, app_root) {
        return Err("That folder stays.".into());
    }
    let meta = std::fs::symlink_metadata(path).map_err(|_| "That is not there.".to_string())?;
    if meta.file_type().is_symlink() || meta.is_file() {
        std::fs::remove_file(path).map_err(|_| "The file did not delete.".to_string())
    } else if meta.is_dir() {
        std::fs::remove_dir_all(path).map_err(|_| "The folder did not delete.".to_string())
    } else {
        Err("That is not there.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_reject_a_slash_and_roots_stay() {
        assert!(safe_name("note").is_some());
        assert!(safe_name("a/b").is_none());
        assert!(safe_name("..").is_none());
        assert!(safe_name("  ").is_none());
        assert!(is_fs_root(Path::new("/")));
        assert!(is_fs_root(Path::new("C:\\")));
        assert!(!is_fs_root(Path::new("/home")));
    }

    #[test]
    fn create_list_and_delete_keep_the_app_folder() {
        let dir = std::env::temp_dir().join(format!("bn-tree-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(protected(&dir, &dir));
        assert!(remove_path(&dir, &dir).is_err());
        let file = create_file(&dir, "note.txt").unwrap();
        assert!(file.is_file());
        assert!(create_file(&dir, "note.txt").is_err());
        let listed = list_dir(&dir).unwrap();
        assert!(listed.iter().any(|e| e.name == "note.txt" && !e.dir));
        let sub = create_dir(&dir, "box").unwrap();
        create_file(&sub, "inside.txt").unwrap();
        assert!(dir_has_child(&sub));
        remove_path(&file, &dir).unwrap();
        assert!(!file.exists());
        remove_path(&sub, &dir).unwrap();
        assert!(!sub.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_parent_of_the_install_stays() {
        let parent = std::env::temp_dir().join(format!("bn-guard-{}", std::process::id()));
        let app = parent.join("blightnet");
        let _ = std::fs::remove_dir_all(&parent);
        std::fs::create_dir_all(&app).unwrap();
        assert!(protected(&parent, &app));
        assert!(protected(&app, &app));
        assert!(remove_path(&parent, &app).is_err());
        assert!(parent.is_dir());
        assert!(app.is_dir());
        let file = create_file(&app, "note.txt").unwrap();
        assert!(!protected(&file, &app));
        remove_path(&file, &app).unwrap();
        assert!(!file.exists());
        let _ = std::fs::remove_dir_all(&parent);
    }
}

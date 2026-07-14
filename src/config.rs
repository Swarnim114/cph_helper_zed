use std::fs;
use std::path::PathBuf;

/// ~/.local/share/cph/
pub fn cph_data_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".local").join("share").join("cph")
    } else {
        PathBuf::from(".")
    }
}

pub fn save_last_problem(path: &PathBuf) {
    let data_dir = cph_data_dir();
    let _ = fs::create_dir_all(&data_dir);
    let _ = fs::write(data_dir.join("last_problem.txt"), path.to_string_lossy().as_bytes());
}

pub fn save_problems_root(path: &PathBuf) {
    let data_dir = cph_data_dir();
    let _ = fs::create_dir_all(&data_dir);
    let _ = fs::write(data_dir.join("problems_root.txt"), path.to_string_lossy().as_bytes());
}

pub fn load_last_problem() -> Option<PathBuf> {
    let path = cph_data_dir().join("last_problem.txt");
    fs::read_to_string(path)
        .ok()
        .map(|s| PathBuf::from(s.trim().to_string()))
        .filter(|p| p.exists())
}

pub fn load_problems_root() -> PathBuf {
    let path = cph_data_dir().join("problems_root.txt");
    fs::read_to_string(path)
        .ok()
        .map(|s| PathBuf::from(s.trim().to_string()))
        .unwrap_or_else(|| PathBuf::from("./problems"))
}

pub fn save_default_language(name: &str) {
    let data_dir = cph_data_dir();
    let _ = fs::create_dir_all(&data_dir);
    let _ = fs::write(data_dir.join("language.txt"), name.as_bytes());
}

/// Load the user's configured default language.
/// Falls back to C++ if nothing has been set or the stored value is unrecognised.
pub fn load_default_language() -> &'static crate::language::LanguageConfig {
    let path = cph_data_dir().join("language.txt");
    let stored = fs::read_to_string(path).unwrap_or_default();
    crate::language::find_language(stored.trim())
        .unwrap_or_else(|| crate::language::find_language("cpp").unwrap())
}

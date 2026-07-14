use std::fs;
use std::path::PathBuf;

// returns the folder where cph stores its data files
// on most linux machines this will be something like /home/yourname/.local/share/cph
fn cph_data_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".local").join("share").join("cph")
}

// saves the path of the last received problem to disk
// this lets "cph-engine run" (with no arguments) know which problem to run
pub fn save_last_problem(path: &PathBuf) {
    let data_dir = cph_data_dir();
    let _ = fs::create_dir_all(&data_dir);
    let _ = fs::write(data_dir.join("last_problem.txt"), path.to_string_lossy().as_bytes());
}

// saves the problems root directory to disk
// this is the folder where all problem folders are created (e.g. ~/cph-engine/problems)
// we save this when the server starts so "run" knows where to look
pub fn save_problems_root(path: &PathBuf) {
    let data_dir = cph_data_dir();
    let _ = fs::create_dir_all(&data_dir);
    let _ = fs::write(data_dir.join("problems_root.txt"), path.to_string_lossy().as_bytes());
}

// reads the last problem path from disk
// returns None if the file doesn't exist or the path no longer exists on disk
pub fn load_last_problem() -> Option<PathBuf> {
    let data_file = cph_data_dir().join("last_problem.txt");

    // try to read the file - if it doesn't exist yet, return nothing
    let content = fs::read_to_string(data_file);
    if content.is_err() {
        return None;
    }

    // turn the stored text into a real path, trimming any extra whitespace
    let path_string = content.unwrap();
    let problem_path = PathBuf::from(path_string.trim());

    // only return the path if the folder still actually exists
    if problem_path.exists() {
        Some(problem_path)
    } else {
        None
    }
}

// reads the problems root directory from disk
// if we've never saved one, fall back to "./problems" in the current directory
pub fn load_problems_root() -> PathBuf {
    let data_file = cph_data_dir().join("problems_root.txt");

    let content = fs::read_to_string(data_file);
    if content.is_err() {
        // first time running, no config saved yet - use a reasonable default
        return PathBuf::from("./problems");
    }

    let path_string = content.unwrap();
    PathBuf::from(path_string.trim())
}

// saves the user's preferred language to disk (e.g. "python", "cpp")
pub fn save_default_language(name: &str) {
    let data_dir = cph_data_dir();
    let _ = fs::create_dir_all(&data_dir);
    let _ = fs::write(data_dir.join("language.txt"), name.as_bytes());
}

// reads the user's preferred language from disk
// if nothing is saved, or the saved language isn't recognised, defaults to C++
pub fn load_default_language() -> &'static crate::language::LanguageConfig {
    let data_file = cph_data_dir().join("language.txt");

    // rule 2: don't use unwrap_or_default() - be explicit about the fallback
    let read_result = fs::read_to_string(data_file);
    let content = if read_result.is_err() {
        String::new() // file doesn't exist yet, treat as empty
    } else {
        read_result.unwrap()
    };

    let lang_name = content.trim();

    // look it up in the language table
    let found = crate::language::find_language(lang_name);

    if let Some(lang) = found {
        lang
    } else {
        // nothing saved yet or unrecognised value - fall back to C++
        crate::language::find_language("cpp").unwrap()
    }
}

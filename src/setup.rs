use std::fs;
use std::path::PathBuf;
use colored::*;

use crate::config::save_default_language;
use crate::language::{find_language, LANGUAGES};

// installs the CPH tasks into Zed's global tasks.json
// after running this, you'll see "CPH: Start Listener" and "CPH: Run Tests" in Zed's command palette
pub fn setup_zed_tasks() {
    // the JSON we'll write into Zed's tasks file
    let tasks_json = r#"[
  {
    "label": "CPH: Start Listener",
    "command": "cph-engine",
    "args": ["serve"],
    "tags": ["cph"]
  },
  {
    "label": "CPH: Run Tests",
    "command": "cph-engine",
    "args": ["run"],
    "tags": ["cph"]
  }
]
"#;

    let config_dir = zed_config_dir();
    let tasks_path = config_dir.join("tasks.json");

    // make sure the Zed config folder exists
    if let Err(e) = fs::create_dir_all(&config_dir) {
        eprintln!("{} Could not create Zed config dir: {}", "Error:".red().bold(), e);
        return;
    }

    // if tasks.json already exists, don't overwrite it - just print what to add manually
    if tasks_path.exists() {
        println!(
            "{} {} already exists. Skipping to avoid overwriting your tasks.",
            "Warning:".yellow().bold(), tasks_path.display()
        );
        println!("Manually add the following to {}:", tasks_path.display());
        println!("{}", tasks_json);
        return;
    }

    // write the tasks file
    match fs::write(&tasks_path, tasks_json) {
        Ok(_) => {
            println!("{} Zed tasks installed to {}", "Done:".green().bold(), tasks_path.display());
            println!("Restart Zed and use the command palette:");
            println!("  CPH: Start Listener");
            println!("  CPH: Run Tests");
        }
        Err(e) => eprintln!("{} Could not write tasks.json: {}", "Error:".red().bold(), e),
    }
}

// returns the path to Zed's config directory
// checks XDG_CONFIG_HOME first, then falls back to ~/.config/zed
fn zed_config_dir() -> PathBuf {
    if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg_config).join("zed");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config").join("zed");
    }
    PathBuf::from(".")
}

// sets the default language for all future problems
// e.g. "cph-engine set-lang python" will make new problems create solution.py
pub fn set_language(lang_str: &str) {
    let found = find_language(lang_str);

    match found {
        Some(lang) => {
            // save the language name to disk so it persists between sessions
            save_default_language(lang.name);
            println!(
                "{} Default language set to {}",
                "Done:".green().bold(),
                lang.display_name.cyan().bold()
            );
            println!("New problems will create: {}", lang.solution_file.yellow());
        }
        None => {
            // the user typed something we don't recognise - show the full list
            eprintln!("{} Unknown language \"{}\"\n", "Error:".red().bold(), lang_str);
            eprintln!("Supported languages:");

            for l in LANGUAGES {
                // build a comma-separated list of the name and all its aliases
                let mut all_names: Vec<&str> = Vec::new();
                all_names.push(l.name);
                for alias in l.aliases {
                    all_names.push(alias);
                }

                eprintln!("  {:<32} {}", all_names.join(", "), l.display_name);
            }
        }
    }
}

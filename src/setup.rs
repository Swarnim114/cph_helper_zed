use std::fs;
use std::path::PathBuf;
use colored::*;

pub fn setup_zed_tasks() {
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

    if let Err(e) = fs::create_dir_all(&config_dir) {
        eprintln!("{} Could not create Zed config dir: {}", "Error:".red().bold(), e);
        return;
    }

    if tasks_path.exists() {
        println!("{} {} already exists. Skipping to avoid overwriting your tasks.",
            "Warning:".yellow().bold(), tasks_path.display());
        println!("Manually add the following to {}:", tasks_path.display());
        println!("{}", tasks_json);
        return;
    }

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

fn zed_config_dir() -> PathBuf {
    if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg_config).join("zed")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config").join("zed")
    } else {
        PathBuf::from(".")
    }
}

use axum::{routing::post, Router, Json};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::io::Write;
use colored::*;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cph-engine")]
#[command(about = "Competitive Programming Helper Engine", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the daemon to listen for Competitive Companion payloads
    Serve {
        #[arg(short, long, default_value_t = 10043)]
        port: u16,
    },
    /// Run tests for a problem.
    /// With no argument, runs the most recently received problem.
    /// With a name, fuzzy-searches for a matching problem.
    ///
    /// Examples:
    ///   cph-engine run
    ///   cph-engine run "number spiral"
    Run {
        /// Problem name to search for (partial, case-insensitive)
        name: Option<String>,
    },
    /// Install CPH tasks into Zed's global tasks.json
    Setup,
}

// Matches the JSON payload sent by the Competitive Companion browser extension
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Problem {
    name: String,
    group: String,
    url: String,
    tests: Vec<TestCase>,
    time_limit: u64,
    memory_limit: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct TestCase {
    input: String,
    output: String,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Serve { port } => serve(*port).await,
        Commands::Run { name } => run_tests(name.as_deref()).await,
        Commands::Setup => setup_zed_tasks(),
    }
}

// ─── Persistence helpers ──────────────────────────────────────────────────────

/// ~/.local/share/cph/
fn cph_data_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".local").join("share").join("cph")
    } else {
        PathBuf::from(".")
    }
}

fn save_last_problem(path: &PathBuf) {
    let data_dir = cph_data_dir();
    let _ = fs::create_dir_all(&data_dir);
    let _ = fs::write(data_dir.join("last_problem.txt"), path.to_string_lossy().as_bytes());
}

fn save_problems_root(path: &PathBuf) {
    let data_dir = cph_data_dir();
    let _ = fs::create_dir_all(&data_dir);
    let _ = fs::write(data_dir.join("problems_root.txt"), path.to_string_lossy().as_bytes());
}

fn load_last_problem() -> Option<PathBuf> {
    let path = cph_data_dir().join("last_problem.txt");
    fs::read_to_string(path)
        .ok()
        .map(|s| PathBuf::from(s.trim().to_string()))
        .filter(|p| p.exists())
}

fn load_problems_root() -> PathBuf {
    let path = cph_data_dir().join("problems_root.txt");
    fs::read_to_string(path)
        .ok()
        .map(|s| PathBuf::from(s.trim().to_string()))
        .unwrap_or_else(|| PathBuf::from("./problems"))
}

// ─── Serve ───────────────────────────────────────────────────────────────────

async fn serve(port: u16) {
    // Remember where problems will be saved so `run` can find them later
    let problems_root = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("problems");
    save_problems_root(&problems_root);

    println!("{}...", "Starting CPH Engine daemon".green().bold());
    println!("Problems will be saved to: {}", problems_root.display());

    let app = Router::new().route("/", post(receive_problem));
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            eprintln!("{} Port {} is already in use.",
                "Error:".red().bold(), port);
            eprintln!("Another instance of cph-engine may already be running.");
            eprintln!("Kill it with: {}", "pkill cph-engine".yellow());
            return;
        }
        Err(e) => {
            eprintln!("{} Could not bind to port {}: {}", "Error:".red().bold(), port, e);
            return;
        }
    };

    println!("Listening for Competitive Companion payloads on http://{}", addr);
    axum::serve(listener, app).await.unwrap();
}

// ─── Problem finder ──────────────────────────────────────────────────────────

/// Fuzzy-search for a problem directory by name.
/// Normalises both the query and directory names (lowercase, spaces→underscores)
/// before matching, then returns the most recently modified match if there are
/// multiple hits.
fn find_problem_by_name(problems_root: &PathBuf, query: &str) -> Option<PathBuf> {
    // Normalise query: lowercase, spaces and hyphens become underscores
    let norm_query = query.to_lowercase().replace([' ', '-'], "_");

    let entries = fs::read_dir(problems_root).ok()?;
    let mut matches: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .to_lowercase()
                .contains(&norm_query)
        })
        .map(|e| e.path())
        .collect();

    if matches.is_empty() {
        return None;
    }

    if matches.len() == 1 {
        return Some(matches.remove(0));
    }

    // Multiple matches — print them and pick the most recently modified
    println!("{} multiple matches found:", "Note:".yellow());
    for m in &matches {
        println!("  - {}", m.display());
    }

    matches.sort_by_key(|p| {
        fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });
    let best = matches.pop().unwrap();
    println!("Running most recent: {}\n", best.file_name().unwrap_or_default().to_string_lossy().cyan());
    Some(best)
}

// ─── Run ─────────────────────────────────────────────────────────────────────

async fn run_tests(name: Option<&str>) {
    let problem_dir = match name {
        // A name was given — search for it
        Some(query) => {
            let problems_root = load_problems_root();
            if !problems_root.exists() {
                eprintln!("{} Problems directory not found at {}. Have you run cph-engine serve yet?",
                    "Error:".red().bold(), problems_root.display());
                return;
            }
            match find_problem_by_name(&problems_root, query) {
                Some(dir) => dir,
                None => {
                    eprintln!("{} No problem matching \"{}\" found in {}",
                        "Error:".red().bold(), query, problems_root.display());
                    return;
                }
            }
        }
        // No name — use the last received problem
        None => {
            match load_last_problem() {
                Some(dir) => {
                    println!("Running latest problem: {}",
                        dir.file_name().unwrap_or_default().to_string_lossy().cyan().bold());
                    dir
                }
                None => {
                    eprintln!("{} No recent problem found.",
                        "Error:".red().bold());
                    eprintln!("Run {} and receive a problem from the browser first.",
                        "cph-engine serve".yellow());
                    return;
                }
            }
        }
    };

    compile_and_run(&problem_dir).await;
}

async fn compile_and_run(problem_dir: &PathBuf) {
    let cph_dir = problem_dir.join(".cph");
    let solution_file = problem_dir.join("solution.cpp");
    let executable = problem_dir.join("solution");

    if !solution_file.exists() {
        eprintln!("{} No solution.cpp found in {}",
            "Error:".red().bold(), problem_dir.display());
        return;
    }

    println!("Compiling: {}", solution_file.display().to_string().yellow());

    let compile_output = Command::new("g++")
        .arg("-O2")
        .arg("-std=c++17")
        .arg(&solution_file)
        .arg("-o")
        .arg(&executable)
        .output();

    match compile_output {
        Ok(out) if out.status.success() => {
            println!("{}", "Compilation successful!".green());
        }
        Ok(out) => {
            eprintln!("{}", "Compilation failed!".red().bold());
            if !out.stderr.is_empty() {
                eprintln!("{}", String::from_utf8_lossy(&out.stderr));
            }
            return;
        }
        Err(e) => {
            eprintln!("{} Could not run g++: {}", "Error:".red().bold(), e);
            eprintln!("Make sure g++ is installed: {}", "sudo pacman -S gcc".yellow());
            return;
        }
    }

    if !executable.exists() {
        eprintln!("{} Compiled binary not found at {}",
            "Error:".red().bold(), executable.display());
        return;
    }

    if !cph_dir.exists() {
        eprintln!("{} No .cph test directory found in {}",
            "Error:".red().bold(), problem_dir.display());
        return;
    }

    let mut i = 1;
    let mut all_passed = true;

    loop {
        let input_path  = cph_dir.join(format!("test_{}.in", i));
        let output_path = cph_dir.join(format!("test_{}.out", i));

        if !input_path.exists() || !output_path.exists() {
            if i == 1 { println!("No test cases found in .cph/"); }
            break;
        }

        let input           = fs::read_to_string(&input_path).unwrap_or_default();
        let expected_output = fs::read_to_string(&output_path).unwrap_or_default();

        print!("Test {} ... ", i);
        // Flush so the label appears before the result
        let _ = std::io::Write::flush(&mut std::io::stdout());

        let mut child = match Command::new(&executable)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                println!("{}", "ERROR".red().bold());
                eprintln!("  Failed to run binary: {}", e);
                all_passed = false;
                i += 1;
                continue;
            }
        };

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(input.as_bytes());
            // stdin is dropped here, closing the pipe
        }

        let output = child.wait_with_output().expect("Failed to read process output");
        let actual_output   = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr_output   = String::from_utf8_lossy(&output.stderr).to_string();

        let actual_trimmed   = actual_output.trim();
        let expected_trimmed = expected_output.trim();

        if actual_trimmed == expected_trimmed {
            println!("{}", "PASS".green().bold());
        } else {
            println!("{}", "FAIL".red().bold());
            println!("  {}", "Expected:".yellow());
            for line in expected_trimmed.lines() {
                println!("    {}", line);
            }
            println!("  {}", "Got:".yellow());
            if actual_trimmed.is_empty() {
                println!("    {}", "(no output)".dimmed());
            } else {
                for line in actual_trimmed.lines() {
                    println!("    {}", line);
                }
            }
            // Print stderr if present (runtime errors, crashes, etc.)
            if !stderr_output.trim().is_empty() {
                println!("  {}", "Stderr:".red());
                for line in stderr_output.trim().lines() {
                    println!("    {}", line);
                }
            }
            // Print exit code if non-zero
            if !output.status.success() {
                println!("  {} {}", "Exit code:".dimmed(),
                    output.status.code().unwrap_or(-1).to_string().red());
            }
            all_passed = false;
        }

        i += 1;
    }

    println!();
    if i > 1 {
        if all_passed {
            println!("{}", "All tests passed!".green().bold());
        } else {
            println!("{}", "Some tests failed.".red().bold());
        }
    }
}

// ─── Receive problem (HTTP handler) ──────────────────────────────────────────

async fn receive_problem(Json(problem): Json<Problem>) {
    println!("--------------------------------------------------");
    println!(" Received: {}", problem.name.cyan().bold());
    println!(" URL:      {}", problem.url);
    println!(" Tests:    {}", problem.tests.len());

    let problems_root = load_problems_root();
    let dir_name = sanitize_name(&problem.name);
    let base_dir = problems_root.join(&dir_name);
    let cph_dir  = base_dir.join(".cph");

    if let Err(e) = fs::create_dir_all(&cph_dir) {
        eprintln!("{} {}", "Failed to create directories:".red(), e);
        return;
    }

    // Track this as the latest problem
    save_last_problem(&base_dir);

    // solution.cpp starter
    let starter_file = base_dir.join("solution.cpp");
    if !starter_file.exists() {
        let template = "#include <iostream>\nusing namespace std;\n\nint main() {\n    \n    return 0;\n}\n";
        match fs::write(&starter_file, template) {
            Ok(_)  => println!(" Created: {}", starter_file.display()),
            Err(e) => eprintln!("{} {}", "Failed to write solution.cpp:".red(), e),
        }
    }

    // README.md
    let readme_file = base_dir.join("README.md");
    if !readme_file.exists() {
        let readme = format!(
            "# {}\n\n**Link:** {}\n**Group:** {}\n**Time Limit:** {} ms\n**Memory Limit:** {} MB\n\n## Test Cases\n\n{} test case(s) loaded.\n",
            problem.name, problem.url, problem.group,
            problem.time_limit, problem.memory_limit,
            problem.tests.len()
        );
        match fs::write(&readme_file, readme) {
            Ok(_)  => println!(" Created: {}", readme_file.display()),
            Err(e) => eprintln!("{} {}", "Failed to write README.md:".red(), e),
        }
    }

    // Test cases
    for (i, test) in problem.tests.iter().enumerate() {
        let _ = fs::write(cph_dir.join(format!("test_{}.in",  i + 1)), &test.input);
        let _ = fs::write(cph_dir.join(format!("test_{}.out", i + 1)), &test.output);
    }

    println!(" Tests saved to: {}", cph_dir.display());
    println!("--------------------------------------------------");
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Replace anything that isn't alphanumeric or a hyphen with an underscore,
/// and strip leading/trailing underscores.
fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn setup_zed_tasks() {
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

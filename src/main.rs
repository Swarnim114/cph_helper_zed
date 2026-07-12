use axum ::{
  routing::post,
  Router,
  Json
};

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
    /// Run tests for a specific problem directory
    Run {
        /// The problem directory containing solution.cpp and .cph/
        #[arg(default_value = ".")]
        dir: String,
    },
}

// This struct matches the JSON payload sent by the Competitive Companion browser extension
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
        Commands::Serve { port } => {
            serve(*port).await;
        }
        Commands::Run { dir } => {
            run_tests(dir).await;
        }
    }
}

async fn serve(port: u16) {
    println!("{}...", "Starting CPH Engine daemon".green().bold());

    // The browser extension always sends the problem data via a POST request to the root path
    let app = Router::new().route("/", post(receive_problem));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Listening for Competitive Companion payloads on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn run_tests(dir: &str) {
    let base_dir = PathBuf::from(dir);
    let cph_dir = base_dir.join(".cph");
    let solution_file = base_dir.join("solution.cpp");
    let executable = base_dir.join("solution");

    if !solution_file.exists() {
        eprintln!("{} Cannot find solution.cpp in {}", "Error:".red().bold(), base_dir.display());
        return;
    }

    println!("{}", "Compiling solution.cpp...".yellow());
    let compile_status = Command::new("g++")
        .arg("-O2")
        .arg("-std=c++17")
        .arg(&solution_file)
        .arg("-o")
        .arg(&executable)
        .status();

    match compile_status {
        Ok(status) if status.success() => {
            println!("{}", "Compilation successful!".green());
        }
        _ => {
            eprintln!("{}", "Compilation failed!".red().bold());
            return;
        }
    }

    if !cph_dir.exists() {
        eprintln!("{} .cph directory not found", "Error:".red().bold());
        return;
    }

    let mut i = 1;
    let mut all_passed = true;
    loop {
        let input_path = cph_dir.join(format!("test_{}.in", i));
        let output_path = cph_dir.join(format!("test_{}.out", i));

        if !input_path.exists() || !output_path.exists() {
            if i == 1 {
                println!("No test cases found.");
            }
            break;
        }

        let input = fs::read_to_string(&input_path).unwrap_or_default();
        let expected_output = fs::read_to_string(&output_path).unwrap_or_default();

        print!("Test {} ... ", i);

        let mut child = Command::new(&executable)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn process");

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(input.as_bytes()).expect("Failed to write to stdin");
        }

        let output = child.wait_with_output().expect("Failed to read stdout");
        let actual_output = String::from_utf8_lossy(&output.stdout).to_string();

        let actual_trimmed = actual_output.trim();
        let expected_trimmed = expected_output.trim();

        if actual_trimmed == expected_trimmed {
            println!("{}", "PASS".green().bold());
        } else {
            println!("{}", "FAIL".red().bold());
            println!("  {}", "Expected:".yellow());
            println!("    {}", expected_trimmed.replace("\n", "\n    "));
            println!("  {}", "Actual:".yellow());
            println!("    {}", actual_trimmed.replace("\n", "\n    "));
            all_passed = false;
        }
        i += 1;
    }

    println!();
    if all_passed && i > 1 {
        println!("{}", "All tests passed! \u{1F389}".green().bold());
    } else if i > 1 {
        println!("{}", "Some tests failed.".red().bold());
    }
}

// This handler receives the JSON payload from the browser
async fn receive_problem(Json(problem): Json<Problem>) {
    println!("--------------------------------------------------");
    println!(" Received new problem: {}", problem.name.cyan().bold());
    println!(" URL: {}", problem.url);
    println!(" Test cases: {}", problem.tests.len());

    let problem_dir_name = problem.name.replace(" ", "_").replace("/", "_");
    let base_dir = PathBuf::from(format!("./problems/{}", problem_dir_name));
    let cph_dir = base_dir.join(".cph");

    if let Err(e) = fs::create_dir_all(&cph_dir) {
        eprintln!("{} {}", "Failed to create directories:".red(), e);
        return;
    }

    let starter_file = base_dir.join("solution.cpp");
    if !starter_file.exists() {
        let template = "#include <iostream>\nusing namespace std;\n\nint main() {\n    \n    return 0;\n}\n";
        if let Err(e) = fs::write(&starter_file, template) {
            eprintln!("{} {}", "Failed to write starter file:".red(), e);
        } else {
            println!(" Created starter file: {}", starter_file.display());
        }
    }

    for (i, test) in problem.tests.iter().enumerate() {
        let input_path = cph_dir.join(format!("test_{}.in", i + 1));
        let output_path = cph_dir.join(format!("test_{}.out", i + 1));

        let _ = fs::write(input_path, &test.input);
        let _ = fs::write(output_path, &test.output);
    }

    println!(" Saved test cases to {}", cph_dir.display());
    println!("--------------------------------------------------");
}

use axum::{routing::post, Router, Json};
use std::net::SocketAddr;
use std::fs;
use std::path::PathBuf;
use colored::*;

use crate::models::Problem;
use crate::config::{load_problems_root, save_problems_root, save_last_problem, load_default_language};

// starts the HTTP server that listens for problem data from the browser extension
pub async fn serve(port: u16) {
    // rule 2: don't use unwrap_or_else - be explicit about what happens if it fails
    let current_dir_result = std::env::current_dir();
    let current_dir = if current_dir_result.is_err() {
        PathBuf::from(".") // fall back to current folder if we can't determine the path
    } else {
        current_dir_result.unwrap()
    };
    let problems_root = current_dir.join("problems");
    save_problems_root(&problems_root);

    println!("{}...", "Starting CPH Engine daemon".green().bold());
    println!("Problems will be saved to: {}", problems_root.display());

    // set up the web server with one route: POST / receives a problem payload
    let app = Router::new().route("/", post(receive_problem));
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    // try to open the port - give a helpful message if something is already using it
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            eprintln!("{} Port {} is already in use.", "Error:".red().bold(), port);
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

// this function runs every time the browser extension sends a problem
// it creates the folder structure and writes the starter files
async fn receive_problem(Json(problem): Json<Problem>) {
    println!("--------------------------------------------------");
    println!(" Received: {}", problem.name.cyan().bold());
    println!(" URL:      {}", problem.url);
    println!(" Tests:    {}", problem.tests.len());

    // figure out where to put this problem
    let problems_root = load_problems_root();
    let dir_name      = sanitize_name(&problem.name);
    let base_dir      = problems_root.join(&dir_name);
    let cph_dir       = base_dir.join(".cph"); // hidden folder that holds test cases

    // create the folders if they don't exist yet
    if let Err(e) = fs::create_dir_all(&cph_dir) {
        eprintln!("{} {}", "Failed to create directories:".red(), e);
        return;
    }

    // remember this as the last received problem so "cph-engine run" can find it
    save_last_problem(&base_dir);

    // create the starter solution file in whatever language the user has set
    let lang = load_default_language();
    let starter_file = base_dir.join(lang.solution_file);
    if !starter_file.exists() {
        match fs::write(&starter_file, lang.template) {
            Ok(_)  => println!(" Created: {} ({})", starter_file.display(), lang.display_name),
            Err(e) => eprintln!("{} {}", "Failed to write solution file:".red(), e),
        }
    }

    // create a README so the user can quickly see the problem details
    let readme_file = base_dir.join("README.md");
    if !readme_file.exists() {
        let readme_content = format!(
            "# {}\n\n**Link:** {}\n**Group:** {}\n**Time Limit:** {} ms\n**Memory Limit:** {} MB\n\n## Test Cases\n\n{} test case(s) loaded.\n",
            problem.name,
            problem.url,
            problem.group,
            problem.time_limit,
            problem.memory_limit,
            problem.tests.len()
        );
        match fs::write(&readme_file, readme_content) {
            Ok(_)  => println!(" Created: {}", readme_file.display()),
            Err(e) => eprintln!("{} {}", "Failed to write README.md:".red(), e),
        }
    }

    // write each test case as a pair of .in and .out files
    for (i, test) in problem.tests.iter().enumerate() {
        let input_file  = cph_dir.join(format!("test_{}.in",  i + 1));
        let output_file = cph_dir.join(format!("test_{}.out", i + 1));
        let _ = fs::write(input_file,  &test.input);
        let _ = fs::write(output_file, &test.output);
    }

    println!(" Tests saved to: {}", cph_dir.display());
    println!("--------------------------------------------------");
}

// converts a problem name into a safe folder name
// replaces anything that isn't a letter, number, or hyphen with an underscore
// e.g. "Two Sets!" becomes "Two_Sets_"
fn sanitize_name(name: &str) -> String {
    let mut result = String::new();
    for ch in name.chars() {
        if ch.is_alphanumeric() || ch == '-' {
            result.push(ch);
        } else {
            result.push('_');
        }
    }
    // remove any underscores from the very start and end
    result.trim_matches('_').to_string()
}

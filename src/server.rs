use axum::{routing::post, Router, Json};
use std::net::SocketAddr;
use std::fs;
use std::path::PathBuf;
use colored::*;

use crate::models::Problem;
use crate::config::{load_problems_root, save_problems_root, save_last_problem};

pub async fn serve(port: u16) {
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

/// Replace anything that isn't alphanumeric or a hyphen with an underscore,
/// and strip leading/trailing underscores.
fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

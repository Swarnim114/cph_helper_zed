use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::io::Write;
use colored::*;

use crate::config::{load_problems_root, load_last_problem};

pub async fn run_tests(name: Option<&str>) {
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

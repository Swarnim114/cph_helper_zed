use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::io::Write;
use colored::*;

use crate::config::{load_problems_root, load_last_problem};
use crate::language::{detect_language, resolve_command, LANGUAGES};

pub async fn run_tests(name: Option<&str>) {
    let problem_dir = match name {
        Some(query) => {
            let problems_root = load_problems_root();
            if !problems_root.exists() {
                eprintln!(
                    "{} Problems directory not found at {}. Have you run cph-engine serve yet?",
                    "Error:".red().bold(), problems_root.display()
                );
                return;
            }
            match find_problem_by_name(&problems_root, query) {
                Some(dir) => dir,
                None => {
                    eprintln!(
                        "{} No problem matching \"{}\" found in {}",
                        "Error:".red().bold(), query, problems_root.display()
                    );
                    return;
                }
            }
        }
        None => match load_last_problem() {
            Some(dir) => {
                println!(
                    "Running latest problem: {}",
                    dir.file_name().unwrap_or_default().to_string_lossy().cyan().bold()
                );
                dir
            }
            None => {
                eprintln!("{} No recent problem found.", "Error:".red().bold());
                eprintln!(
                    "Run {} and receive a problem from the browser first.",
                    "cph-engine serve".yellow()
                );
                return;
            }
        },
    };

    compile_and_run(&problem_dir).await;
}

// ─── Problem finder ───────────────────────────────────────────────────────────

/// Case-insensitive, partial-match search. Spaces and hyphens are normalised to
/// underscores before comparison. When multiple directories match, the most
/// recently modified one wins.
fn find_problem_by_name(problems_root: &PathBuf, query: &str) -> Option<PathBuf> {
    let norm_query = query.to_lowercase().replace([' ', '-'], "_");

    let mut matches: Vec<PathBuf> = fs::read_dir(problems_root)
        .ok()?
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
    println!(
        "Running most recent: {}\n",
        best.file_name().unwrap_or_default().to_string_lossy().cyan()
    );
    Some(best)
}

// ─── Compile & run ────────────────────────────────────────────────────────────

async fn compile_and_run(problem_dir: &PathBuf) {
    let cph_dir = problem_dir.join(".cph");

    // 1. Auto-detect language by checking which solution file exists
    let lang = match detect_language(problem_dir) {
        Some(l) => l,
        None => {
            let known: Vec<&str> = LANGUAGES.iter().map(|l| l.solution_file).collect();
            eprintln!(
                "{} No known solution file found in {}",
                "Error:".red().bold(), problem_dir.display()
            );
            eprintln!("Expected one of: {}", known.join(", "));
            return;
        }
    };

    let solution_file = problem_dir.join(lang.solution_file);
    let bin           = problem_dir.join(lang.bin_name);

    println!("Language:  {}", lang.display_name.cyan().bold());

    // 2. Compile (only for compiled languages; None = interpreted, skip silently)
    if let Some(compile_template) = lang.compile_cmd {
        println!("Compiling: {}", solution_file.display().to_string().yellow());

        let result = resolve_command(compile_template, &solution_file, &bin, problem_dir)
            .output();

        match result {
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
                eprintln!("{} Could not run compiler: {}", "Error:".red().bold(), e);
                return;
            }
        }
    }

    // 3. Verify test directory
    if !cph_dir.exists() {
        eprintln!(
            "{} No .cph test directory found in {}",
            "Error:".red().bold(), problem_dir.display()
        );
        return;
    }

    // 4. Run tests
    let mut i = 1;
    let mut all_passed = true;

    loop {
        let input_path  = cph_dir.join(format!("test_{}.in",  i));
        let output_path = cph_dir.join(format!("test_{}.out", i));

        if !input_path.exists() || !output_path.exists() {
            if i == 1 {
                println!("No test cases found in .cph/");
            }
            break;
        }

        let input           = fs::read_to_string(&input_path).unwrap_or_default();
        let expected_output = fs::read_to_string(&output_path).unwrap_or_default();

        print!("Test {} ... ", i);
        let _ = std::io::Write::flush(&mut std::io::stdout());

        let mut child = match resolve_command(lang.run_cmd, &solution_file, &bin, problem_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                println!("{}", "ERROR".red().bold());
                eprintln!("  Failed to run: {}", e);
                all_passed = false;
                i += 1;
                continue;
            }
        };

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(input.as_bytes());
            // dropping stdin closes the write-end of the pipe
        }

        let output        = child.wait_with_output().expect("Failed to read process output");
        let actual_output = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr_output = String::from_utf8_lossy(&output.stderr).to_string();

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
            if !stderr_output.trim().is_empty() {
                println!("  {}", "Stderr:".red());
                for line in stderr_output.trim().lines() {
                    println!("    {}", line);
                }
            }
            if !output.status.success() {
                println!(
                    "  {} {}",
                    "Exit code:".dimmed(),
                    output.status.code().unwrap_or(-1).to_string().red()
                );
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

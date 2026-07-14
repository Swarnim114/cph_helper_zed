use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::io::Write;
use colored::*;

use crate::config::{load_problems_root, load_last_problem};
use crate::language::{detect_language, resolve_command, LANGUAGES};

// entry point for "cph-engine run" and "cph-engine run <name>"
pub async fn run_tests(name: Option<&str>) {
    let problem_dir = match name {
        // user gave a name, so search for a matching folder
        Some(query) => {
            let problems_root = load_problems_root();

            if !problems_root.exists() {
                eprintln!(
                    "{} Problems directory not found at {}.",
                    "Error:".red().bold(), problems_root.display()
                );
                eprintln!("Have you run {} yet?", "cph-engine serve".yellow());
                return;
            }

            let found = find_problem_by_name(&problems_root, query);
            match found {
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

        // no name given, run the most recently received problem
        None => {
            let last = load_last_problem();
            match last {
                Some(dir) => {
                    // rule 3: break the chain - get name, convert, then print separately
                    let name_option = dir.file_name();
                    let name        = name_option.unwrap_or_default();
                    let name_str    = name.to_string_lossy();
                    println!("Running latest problem: {}", name_str.cyan().bold());
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
            }
        }
    };

    compile_and_run(&problem_dir).await;
}

// searches the problems folder for a directory whose name contains the query
// case-insensitive, spaces treated the same as underscores
// if multiple folders match, the most recently modified one wins
fn find_problem_by_name(problems_root: &PathBuf, query: &str) -> Option<PathBuf> {
    // normalise the query so "number spiral" matches "Number_Spiral"
    let norm_query = query.to_lowercase().replace([' ', '-'], "_");

    // try to open the problems folder for reading
    let entries_result = fs::read_dir(problems_root);
    if entries_result.is_err() {
        return None;
    }

    let mut matches: Vec<PathBuf> = Vec::new();

    for entry_result in entries_result.unwrap() {
        // skip entries we can't read
        if entry_result.is_err() {
            continue;
        }
        let entry = entry_result.unwrap();

        // rule 1: break .map() on Result into explicit steps
        // get the file type first, then check if it's a directory separately
        let file_type_result = entry.file_type();
        if file_type_result.is_err() {
            continue;
        }
        let file_type = file_type_result.unwrap();
        let is_dir    = file_type.is_dir();

        if !is_dir {
            continue;
        }

        // rule 3: break the name chain into separate steps
        let os_name    = entry.file_name();
        let lossy_name = os_name.to_string_lossy();
        let dir_name   = lossy_name.to_lowercase();

        if dir_name.contains(&norm_query) {
            matches.push(entry.path());
        }
    }

    if matches.is_empty() {
        return None;
    }

    if matches.len() == 1 {
        return Some(matches.remove(0));
    }

    // multiple matches - show them all and pick the most recently modified
    println!("{} multiple matches found:", "Note:".yellow());
    for m in &matches {
        println!("  - {}", m.display());
    }

    matches.sort_by_key(|path| {
        // rule 1: break the Result chain - get metadata first, then modified time separately
        let metadata_result = fs::metadata(path);
        if metadata_result.is_err() {
            return std::time::SystemTime::UNIX_EPOCH;
        }

        let metadata        = metadata_result.unwrap();
        let modified_result = metadata.modified();
        if modified_result.is_err() {
            return std::time::SystemTime::UNIX_EPOCH;
        }

        modified_result.unwrap()
    });

    // pop() gives us the last element which is the most recently modified
    let best = matches.pop().unwrap();

    // rule 3: break the display chain into steps
    let best_name     = best.file_name().unwrap_or_default();
    let best_name_str = best_name.to_string_lossy();
    println!("Running most recent: {}\n", best_name_str.cyan());

    Some(best)
}

// compiles the solution (if needed) and runs it against all test cases
async fn compile_and_run(problem_dir: &PathBuf) {
    let cph_dir = problem_dir.join(".cph");

    // figure out which language was used by checking which solution file exists
    let lang_option = detect_language(problem_dir);
    if lang_option.is_none() {
        // build a list of all known solution filenames so we can show them
        let mut known_files: Vec<&str> = Vec::new();
        for l in LANGUAGES {
            known_files.push(l.solution_file);
        }
        eprintln!(
            "{} No known solution file found in {}",
            "Error:".red().bold(), problem_dir.display()
        );
        eprintln!("Expected one of: {}", known_files.join(", "));
        return;
    }
    let lang = lang_option.unwrap();

    let solution_file = problem_dir.join(lang.solution_file);
    let bin           = problem_dir.join(lang.bin_name);

    println!("Language:  {}", lang.display_name.cyan().bold());

    // compile if this language needs it (None means interpreted, skip the compile step)
    if let Some(compile_template) = lang.compile_cmd {
        // rule 3: break display().to_string() into two steps
        let display     = solution_file.display();
        let display_str = display.to_string();
        println!("Compiling: {}", display_str.yellow());

        let compile_result = resolve_command(compile_template, &solution_file, &bin, problem_dir)
            .output();

        match compile_result {
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

    if !cph_dir.exists() {
        eprintln!(
            "{} No .cph test directory found in {}",
            "Error:".red().bold(), problem_dir.display()
        );
        return;
    }

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

        let child_result = resolve_command(lang.run_cmd, &solution_file, &bin, problem_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let mut child = match child_result {
            Ok(c) => c,
            Err(e) => {
                println!("{}", "ERROR".red().bold());
                eprintln!("  Failed to start the program: {}", e);
                all_passed = false;
                i += 1;
                continue;
            }
        };

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(input.as_bytes());
            // dropping stdin closes the pipe and signals EOF to the program
        }

        let output = child.wait_with_output().expect("Failed to read process output");

        // rule 3: break from_utf8_lossy().to_string() into two steps
        let stdout_lossy  = String::from_utf8_lossy(&output.stdout);
        let actual_output = stdout_lossy.to_string();

        let stderr_lossy  = String::from_utf8_lossy(&output.stderr);
        let stderr_output = stderr_lossy.to_string();

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

            // rule 2: break .code().unwrap_or(-1) into an explicit if/else
            if !output.status.success() {
                let code_option = output.status.code();
                let exit_code = if code_option.is_none() {
                    -1
                } else {
                    code_option.unwrap()
                };
                println!("  {} {}", "Exit code:".dimmed(), exit_code.to_string().red());
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

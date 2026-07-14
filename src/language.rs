use std::path::Path;
use std::process::Command;

// This struct holds everything we need to know about one programming language.
// Think of it like a row in a spreadsheet - each field is a column.
// We never need to change this struct to add a new language, we just add a new row below.
#[allow(dead_code)] // extension is kept for possible future use (file pickers etc.)
pub struct LanguageConfig {
    // the short name used on the command line, e.g. "cpp", "python"
    pub name: &'static str,

    // what we show the user in terminal output
    pub display_name: &'static str,

    // other names the user might type for this language
    // e.g. "c++" and "cc" both map to "cpp"
    pub aliases: &'static [&'static str],

    // the file extension without the dot, e.g. "cpp", "py"
    pub extension: &'static str,

    // the actual filename we create in the problem folder
    // java is special because the class name must match the filename
    pub solution_file: &'static str,

    // the starter code we write when a new problem arrives
    pub template: &'static str,

    // the command to compile the solution
    // None means the language is interpreted and has no compile step (like Python)
    // the strings can contain {file}, {bin}, {dir} which get replaced with real paths at runtime
    pub compile_cmd: Option<&'static [&'static str]>,

    // the command to actually run the solution
    // same {file}, {bin}, {dir} placeholders work here too
    pub run_cmd: &'static [&'static str],

    // name of the compiled output file placed in the problem folder
    // usually just "solution", but Kotlin produces "solution.jar"
    pub bin_name: &'static str,
}

// This is the full language table.
// To add a new language, just copy one of these blocks and fill in the details.
// Nothing else in the codebase needs to change.
pub static LANGUAGES: &[LanguageConfig] = &[
    // C++ - the most common language in competitive programming
    LanguageConfig {
        name:          "cpp",
        display_name:  "C++ (g++, C++17)",
        aliases:       &["c++", "cc", "cxx"],
        extension:     "cpp",
        solution_file: "solution.cpp",
        template: r#"#include <bits/stdc++.h>
using namespace std;

int main() {
    ios_base::sync_with_stdio(false);
    cin.tie(NULL);

    return 0;
}
"#,
        compile_cmd: Some(&["g++", "-O2", "-std=c++17", "{file}", "-o", "{bin}"]),
        run_cmd:     &["{bin}"],
        bin_name:    "solution",
    },

    // C - slightly lower level than C++, still popular
    LanguageConfig {
        name:          "c",
        display_name:  "C (gcc, C11)",
        aliases:       &["gcc"],
        extension:     "c",
        solution_file: "solution.c",
        template: r#"#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main() {

    return 0;
}
"#,
        compile_cmd: Some(&["gcc", "-O2", "-std=c11", "-lm", "{file}", "-o", "{bin}"]),
        run_cmd:     &["{bin}"],
        bin_name:    "solution",
    },

    // Python - no compilation needed, runs directly
    LanguageConfig {
        name:          "python",
        display_name:  "Python (python3)",
        aliases:       &["py", "python3"],
        extension:     "py",
        solution_file: "solution.py",
        template: r#"import sys
input = sys.stdin.readline

def main():
    pass

if __name__ == "__main__":
    main()
"#,
        compile_cmd: None, // interpreted, skip compile step
        run_cmd:     &["python3", "{file}"],
        bin_name:    "solution",
    },

    // Java - note: the file must be named Main.java because the class is called Main
    LanguageConfig {
        name:          "java",
        display_name:  "Java (javac + JVM)",
        aliases:       &[],
        extension:     "java",
        solution_file: "Main.java",
        template: r#"import java.util.*;
import java.io.*;

public class Main {
    public static void main(String[] args) throws IOException {
        BufferedReader br = new BufferedReader(new InputStreamReader(System.in));

    }
}
"#,
        compile_cmd: Some(&["javac", "{file}"]),
        run_cmd:     &["java", "-cp", "{dir}", "Main"],
        bin_name:    "solution",
    },

    // Rust - compiled language, fast but stricter than C++
    LanguageConfig {
        name:          "rust",
        display_name:  "Rust (rustc)",
        aliases:       &["rs"],
        extension:     "rs",
        solution_file: "solution.rs",
        template: r#"use std::io::{self, BufRead, Write};

fn main() {
    let stdin  = io::stdin();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    for line in stdin.lock().lines() {
        let _line = line.unwrap();
        writeln!(out, "").unwrap();
    }
}
"#,
        compile_cmd: Some(&["rustc", "-O", "{file}", "-o", "{bin}"]),
        run_cmd:     &["{bin}"],
        bin_name:    "solution",
    },

    // Go - interpreted via "go run", no separate compile step needed
    LanguageConfig {
        name:          "go",
        display_name:  "Go (go run)",
        aliases:       &["golang"],
        extension:     "go",
        solution_file: "solution.go",
        template: r#"package main

import (
    "bufio"
    "fmt"
    "os"
)

var reader *bufio.Reader
var writer *bufio.Writer

func main() {
    reader = bufio.NewReader(os.Stdin)
    writer = bufio.NewWriter(os.Stdout)
    defer writer.Flush()

    _ = fmt.Fscan
    _ = reader
}
"#,
        compile_cmd: None, // "go run" handles compilation internally
        run_cmd:     &["go", "run", "{file}"],
        bin_name:    "solution",
    },

    // JavaScript - runs with Node.js, no compilation needed
    LanguageConfig {
        name:          "javascript",
        display_name:  "JavaScript (Node.js)",
        aliases:       &["js", "node"],
        extension:     "js",
        solution_file: "solution.js",
        template: r#"const readline = require('readline');
const rl = readline.createInterface({ input: process.stdin });
const lines = [];
rl.on('line', line => lines.push(line.trim()));
rl.on('close', () => {
    let idx = 0;
    const next = () => lines[idx++];
    // Your solution here
    void next;
});
"#,
        compile_cmd: None, // interpreted, skip compile step
        run_cmd:     &["node", "{file}"],
        bin_name:    "solution",
    },

    // Kotlin - compiles to a .jar file, then run with java -jar
    LanguageConfig {
        name:          "kotlin",
        display_name:  "Kotlin (kotlinc + JVM)",
        aliases:       &["kt"],
        extension:     "kt",
        solution_file: "solution.kt",
        template: r#"import java.util.Scanner

fun main() {
    val sc = Scanner(System.`in`)

}
"#,
        compile_cmd: Some(&["kotlinc", "{file}", "-include-runtime", "-d", "{bin}"]),
        run_cmd:     &["java", "-jar", "{bin}"],
        bin_name:    "solution.jar", // kotlin outputs a jar, not a plain binary
    },
];

// looks up a language by name or alias
// the search is case-insensitive, so "CPP" and "cpp" both work
pub fn find_language(name: &str) -> Option<&'static LanguageConfig> {
    let lower = name.to_lowercase();

    for lang in LANGUAGES {
        // check the main name first
        if lang.name == lower {
            return Some(lang);
        }

        // check any aliases (e.g. "c++" is an alias for "cpp")
        for alias in lang.aliases {
            if *alias == lower {
                return Some(lang);
            }
        }
    }

    // no match found
    None
}

// looks inside a problem directory to figure out which language was used
// it does this by checking which solution file exists (solution.cpp, solution.py, etc.)
pub fn detect_language(problem_dir: &Path) -> Option<&'static LanguageConfig> {
    for lang in LANGUAGES {
        let solution_path = problem_dir.join(lang.solution_file);
        if solution_path.exists() {
            return Some(lang);
        }
    }

    // none of the known solution files were found
    None
}

// takes a command template like ["g++", "{file}", "-o", "{bin}"]
// and fills in the real file paths before returning a ready-to-run Command
pub fn resolve_command(template: &[&str], file: &Path, bin: &Path, dir: &Path) -> Command {
    // rule 3: convert each path in two steps - lossy first, then to owned String
    let file_lossy = file.to_string_lossy();
    let file_str   = file_lossy.to_string();

    let bin_lossy = bin.to_string_lossy();
    let bin_str   = bin_lossy.to_string();

    let dir_lossy = dir.to_string_lossy();
    let dir_str   = dir_lossy.to_string();

    // go through each word in the template and replace the placeholders with real paths
    let mut resolved: Vec<String> = Vec::new();
    for arg in template {
        // rule 4: do each replacement on a separate line, not chained
        let step1 = arg.replace("{file}", &file_str);
        let step2 = step1.replace("{bin}",  &bin_str);
        let filled = step2.replace("{dir}",  &dir_str);
        resolved.push(filled);
    }

    // the first word is the program to run, everything after it are the arguments
    let mut cmd = Command::new(&resolved[0]);
    for arg in &resolved[1..] {
        cmd.arg(arg);
    }
    cmd
}

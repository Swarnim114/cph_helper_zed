use std::path::Path;
use std::process::Command;

/// A complete, self-contained description of how to handle a programming language.
/// This is pure data — no methods on this type. All behaviour lives in the free
/// functions below (find_language, detect_language, resolve_command).
#[allow(dead_code)] // `extension` is kept for future use (file pickers, display, etc.)
pub struct LanguageConfig {
    /// Canonical name used in the config file and CLI (e.g. "cpp", "python")
    pub name: &'static str,
    /// Human-readable label shown in terminal output
    pub display_name: &'static str,
    /// Alternative names the user may type (e.g. "c++", "py")
    pub aliases: &'static [&'static str],
    /// File extension without the dot (e.g. "cpp", "py")
    pub extension: &'static str,
    /// Exact filename created inside the problem directory
    pub solution_file: &'static str,
    /// Boilerplate starter code written when a problem is received
    pub template: &'static str,
    /// Compilation command template.
    ///   None  => interpreted language, skip compile step entirely.
    ///   Some  => list of args; supports placeholders {file}, {bin}, {dir}.
    pub compile_cmd: Option<&'static [&'static str]>,
    /// Execution command template. Supports the same placeholders.
    pub run_cmd: &'static [&'static str],
    /// Name of the compiled artifact placed in the problem directory.
    /// Usually "solution", but "solution.jar" for Kotlin, etc.
    pub bin_name: &'static str,
}

// ─── Language table ───────────────────────────────────────────────────────────
// To add a new language: append one LanguageConfig block below. Nothing else
// in the codebase needs to change.

pub static LANGUAGES: &[LanguageConfig] = &[
    // ── C++ ──────────────────────────────────────────────────────────────────
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

    // ── C ────────────────────────────────────────────────────────────────────
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

    // ── Python ───────────────────────────────────────────────────────────────
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
        compile_cmd: None,
        run_cmd:     &["python3", "{file}"],
        bin_name:    "solution",
    },

    // ── Java ─────────────────────────────────────────────────────────────────
    LanguageConfig {
        name:          "java",
        display_name:  "Java (javac + JVM)",
        aliases:       &[],
        extension:     "java",
        solution_file: "Main.java",     // Java class name must match filename
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

    // ── Rust ─────────────────────────────────────────────────────────────────
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

    // ── Go ───────────────────────────────────────────────────────────────────
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
        compile_cmd: None,
        run_cmd:     &["go", "run", "{file}"],
        bin_name:    "solution",
    },

    // ── JavaScript (Node.js) ─────────────────────────────────────────────────
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
        compile_cmd: None,
        run_cmd:     &["node", "{file}"],
        bin_name:    "solution",
    },

    // ── Kotlin ───────────────────────────────────────────────────────────────
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
        // kotlinc outputs a fat JAR; bin_name reflects that
        compile_cmd: Some(&["kotlinc", "{file}", "-include-runtime", "-d", "{bin}"]),
        run_cmd:     &["java", "-jar", "{bin}"],
        bin_name:    "solution.jar",
    },
];

// ─── Free functions ───────────────────────────────────────────────────────────

/// Find a language by name or alias (case-insensitive).
/// Returns None if no match is found — callers decide how to handle that.
pub fn find_language(name: &str) -> Option<&'static LanguageConfig> {
    let lower = name.to_lowercase();
    LANGUAGES
        .iter()
        .find(|l| l.name == lower || l.aliases.contains(&lower.as_str()))
}

/// Scan a problem directory for a known solution file and return the matching
/// language config. Returns the first match in LANGUAGES order (C++ wins ties).
pub fn detect_language(problem_dir: &Path) -> Option<&'static LanguageConfig> {
    LANGUAGES
        .iter()
        .find(|l| problem_dir.join(l.solution_file).exists())
}

/// Substitute {file}, {bin}, {dir} placeholders in a command template and
/// return a ready-to-spawn Command. The first element is the executable name.
pub fn resolve_command(template: &[&str], file: &Path, bin: &Path, dir: &Path) -> Command {
    let file_str = file.to_string_lossy().to_string();
    let bin_str  = bin.to_string_lossy().to_string();
    let dir_str  = dir.to_string_lossy().to_string();

    let resolved: Vec<String> = template
        .iter()
        .map(|&arg| {
            arg.replace("{file}", &file_str)
               .replace("{bin}",  &bin_str)
               .replace("{dir}",  &dir_str)
        })
        .collect();

    let mut cmd = Command::new(&resolved[0]);
    if resolved.len() > 1 {
        cmd.args(&resolved[1..]);
    }
    cmd
}

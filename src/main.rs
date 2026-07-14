// each module is a separate file in src/
// split this way so each file has one clear job
mod config;   // reading and writing settings to disk
mod language; // the table of supported programming languages
mod models;   // the shape of JSON data we receive from the browser extension
mod runner;   // compiling and running test cases
mod server;   // the HTTP server that listens for the browser extension
mod setup;    // installing Zed tasks and changing language settings

use clap::{Parser, Subcommand};

// this is the top-level CLI definition
// clap reads this struct and automatically generates --help and argument parsing
#[derive(Parser)]
#[command(name = "cph-engine")]
#[command(about = "Competitive Programming Helper Engine", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

// each variant here is one subcommand (e.g. "cph-engine serve", "cph-engine run")
#[derive(Subcommand)]
enum Commands {
    /// Start the daemon - listens for problems sent by the browser extension
    Serve {
        /// which port to listen on (default: 10043, matches Competitive Companion)
        #[arg(short, long, default_value_t = 10043)]
        port: u16,
    },

    /// Run tests for a problem.
    ///
    /// With no argument: runs the most recently received problem.
    /// With a name: fuzzy-searches for a matching problem folder.
    ///
    /// Examples:
    ///   cph-engine run
    ///   cph-engine run "number spiral"
    ///   cph-engine run spiral
    Run {
        /// part of the problem name to search for (case-insensitive)
        name: Option<String>,
    },

    /// Install CPH tasks into Zed's global tasks.json
    Setup,

    /// Set the default language for new problems.
    ///
    /// Examples:
    ///   cph-engine set-lang python
    ///   cph-engine set-lang cpp
    ///   cph-engine set-lang rust
    #[command(name = "set-lang")]
    SetLang {
        /// language name or alias (cpp, c++, python, py, java, rust, go, js, kotlin...)
        language: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // hand off to the right module based on which subcommand was typed
    match &cli.command {
        Commands::Serve { port }       => server::serve(*port).await,
        Commands::Run { name }         => runner::run_tests(name.as_deref()).await,
        Commands::Setup                => setup::setup_zed_tasks(),
        Commands::SetLang { language } => setup::set_language(language),
    }
}

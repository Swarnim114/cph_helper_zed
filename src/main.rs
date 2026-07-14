mod config;
mod language;
mod models;
mod runner;
mod server;
mod setup;

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
    /// Set the default language for new problems.
    ///
    /// Examples:
    ///   cph-engine set-lang python
    ///   cph-engine set-lang cpp
    ///   cph-engine set-lang rust
    #[command(name = "set-lang")]
    SetLang {
        /// Language name or alias (cpp, c++, python, py, java, rust, go, js, kotlin, ...)
        language: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Serve { port } => server::serve(*port).await,
        Commands::Run { name }   => runner::run_tests(name.as_deref()).await,
        Commands::Setup          => setup::setup_zed_tasks(),
        Commands::SetLang { language } => setup::set_language(language),
    }
}

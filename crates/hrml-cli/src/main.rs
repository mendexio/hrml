use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "hrml")]
#[command(about = "HRML — Hypertext Reactive Markup Language compiler")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compile .hrml files to HTML + CSS + JS
    Build {
        /// Input file or directory
        path: String,
    },

    /// Watch for changes and recompile
    Dev {
        /// Input file or directory
        path: String,
    },

    /// Check .hrml files for errors without generating output
    Check {
        /// Input file or directory
        path: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Build { path } => {
            eprintln!("hrml build: not implemented yet (path: {path})");
            std::process::exit(1);
        }
        Command::Dev { path } => {
            eprintln!("hrml dev: not implemented yet (path: {path})");
            std::process::exit(1);
        }
        Command::Check { path } => {
            eprintln!("hrml check: not implemented yet (path: {path})");
            std::process::exit(1);
        }
    }
}

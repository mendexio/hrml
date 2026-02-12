use clap::{Parser, Subcommand};
use std::path::Path;

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
    /// Compile an .hrml file to HTML + CSS + JS
    Build {
        /// Input .hrml file
        path: String,
    },

    /// Check an .hrml file for errors without generating output
    Check {
        /// Input .hrml file
        path: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Build { path } => cmd_build(&path),
        Command::Check { path } => cmd_check(&path),
    }
}

fn read_source(path: &str) -> String {
    let p = Path::new(path);
    if !p.exists() {
        eprintln!("Error: file not found: {path}");
        std::process::exit(1);
    }
    match std::fs::read_to_string(p) {
        Ok(source) => source,
        Err(e) => {
            eprintln!("Error reading {path}: {e}");
            std::process::exit(1);
        }
    }
}

fn cmd_build(path: &str) {
    let source = read_source(path);

    let doc = match hrml_parser::Parser::parse(&source) {
        Ok(doc) => doc,
        Err(e) => {
            eprintln!("Parse error: {e}");
            std::process::exit(1);
        }
    };

    let output = match hrml_codegen::compile(&doc) {
        Ok(output) => output,
        Err(e) => {
            eprintln!("Codegen error: {e}");
            std::process::exit(1);
        }
    };

    // Write output files next to the source
    let stem = Path::new(path).file_stem().unwrap().to_str().unwrap();
    let dir = Path::new(path).parent().unwrap_or(Path::new("."));

    let html_path = dir.join(format!("{stem}.html"));
    let js_path = dir.join(format!("{stem}.js"));

    // Build a standalone HTML file
    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html>\n<head>\n  <meta charset=\"UTF-8\">\n");
    html.push_str(&format!("  <title>{stem}</title>\n"));
    if !output.css.is_empty() {
        html.push_str(&format!("  <style>\n{}\n  </style>\n", output.css));
    }
    html.push_str("</head>\n<body>\n");
    html.push_str(&output.html);
    if !output.js.is_empty() {
        html.push_str(&format!("<script>\n{}</script>\n", output.js));
    }
    html.push_str("</body>\n</html>\n");

    if let Err(e) = std::fs::write(&html_path, &html) {
        eprintln!("Error writing {}: {e}", html_path.display());
        std::process::exit(1);
    }

    // Also write standalone JS if non-empty
    if !output.js.is_empty() {
        if let Err(e) = std::fs::write(&js_path, &output.js) {
            eprintln!("Error writing {}: {e}", js_path.display());
            std::process::exit(1);
        }
    }

    eprintln!("Built: {}", html_path.display());
}

fn cmd_check(path: &str) {
    let source = read_source(path);

    if let Err(e) = hrml_parser::Parser::parse(&source) {
        eprintln!("Parse error: {e}");
        std::process::exit(1);
    }

    // Also try codegen to catch codegen errors
    let doc = hrml_parser::Parser::parse(&source).unwrap();
    if let Err(e) = hrml_codegen::compile(&doc) {
        eprintln!("Codegen error: {e}");
        std::process::exit(1);
    }

    eprintln!("OK: {path}");
}

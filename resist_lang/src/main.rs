//! `resist_cli` — Execute ResistScript v2 (.res) files from the command line.

use std::env;
use std::fs;
use resist_lang::eval_api::eval_script;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: resist_cli <script.res>");
        std::process::exit(1);
    }

    let source = match fs::read_to_string(&args[1]) {
        Ok(s) => s,
        Err(e) => { eprintln!("Error reading '{}': {}", args[1], e); std::process::exit(1); }
    };

    println!("╔══════════════════════════════════════════════════╗");
    println!("║       ⚡ ResistScript v2 Interpreter            ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!("  File: {}\n", args[1]);

    match eval_script(&source) {
        Ok(result) => {
            for line in &result.log {
                println!("{}", line);
            }
            println!("\n── Summary ──────────────────────────────────────");
            println!("  Components: {}", result.layout.len());
            println!("  Nodes:      {}", result.nodes.len());
            println!("  Analyses:   {}", result.analyses.len());
            println!("  Done. ✓");
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

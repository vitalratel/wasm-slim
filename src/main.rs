use clap::{Parser, Subcommand};
use clap_complete::Shell;
use std::process;
use wasm_slim::cmd;

/// WASM bundle size optimizer
///
/// wasm-slim automates the complex process of optimizing WASM binary sizes,
/// providing 60%+ size reductions without requiring deep expertise.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Disable emoji output (useful for CI/CD or accessibility)
    #[arg(long, global = true)]
    no_emoji: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Build and optimize WASM binary
    Build {
        /// Run in dry-run mode (show what would be done without making changes)
        #[arg(short, long)]
        dry_run: bool,

        /// Check bundle size against budget (fail if exceeded)
        #[arg(long)]
        check: bool,

        /// Output as JSON (for CI/CD integration)
        #[arg(long)]
        json: bool,

        /// Target directory for output
        #[arg(short, long)]
        target_dir: Option<String>,
    },

    /// Analyze WASM bundle or dependencies
    Analyze {
        /// WASM file to analyze (optional, omit for dependency analysis)
        #[arg(value_name = "FILE")]
        file: Option<String>,

        /// Analysis mode: assets, deps, top, dominators, dead
        #[arg(short, long, default_value = "deps")]
        mode: String,

        /// Automatically apply optimization suggestions to Cargo.toml
        #[arg(long)]
        fix: bool,

        /// Show what would be changed without modifying files
        #[arg(long)]
        dry_run: bool,

        /// Show externalization guide (for assets mode)
        #[arg(long)]
        guide: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Initialize wasm-slim configuration
    Init {
        /// Template to use: minimal, balanced, aggressive
        #[arg(short, long, default_value = "balanced")]
        template: String,
    },

    /// Compare two WASM builds
    Compare {
        /// Before file
        before: String,

        /// After file
        after: String,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

fn main() {
    // Initialize logger (use RUST_LOG env var to control verbosity)
    env_logger::init();

    let cli = Cli::parse();

    // Set console emoji mode based on CLI flag
    if cli.no_emoji {
        std::env::set_var("NO_EMOJI", "1");
    }

    let result = match &cli.command {
        Some(Commands::Build {
            dry_run,
            check,
            json,
            target_dir,
        }) => cmd::cmd_build(*dry_run, *check, *json, target_dir.as_deref()),
        Some(Commands::Analyze {
            file,
            mode,
            fix,
            dry_run,
            guide,
            json,
        }) => cmd::cmd_analyze(file, mode, *fix, *dry_run, *guide, *json),
        Some(Commands::Init { template }) => cmd::cmd_init(template),
        Some(Commands::Compare { before, after }) => cmd::cmd_compare(before, after),
        Some(Commands::Completions { shell }) => {
            cmd::cmd_completions(*shell);
            Ok(())
        }
        None => {
            // No subcommand provided, show help
            println!("wasm-slim v{}", env!("CARGO_PKG_VERSION"));
            println!("WASM bundle size optimizer\n");
            println!("Usage: wasm-slim <COMMAND>\n");
            println!("Commands:");
            println!("  build    Build and optimize WASM binary");
            println!("  analyze  Analyze WASM bundle size");
            println!("  init     Initialize wasm-slim configuration");
            println!("  compare  Compare two WASM builds");
            println!("\nRun 'wasm-slim <COMMAND> --help' for more information on a command.");
            Ok(())
        }
    };

    if let Err(e) = result {
        use wasm_slim::error::ErrorFormatter;
        eprintln!("{}", ErrorFormatter::format(&e));
        let exit_code = ErrorFormatter::exit_code(&e);
        process::exit(exit_code);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert()
    }
}

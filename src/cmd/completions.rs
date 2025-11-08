//! Completions command implementation
//!
//! Handles the `wasm-slim completions` command which generates
//! shell completion scripts for bash, zsh, fish, etc.

use clap_complete::{generate, Shell};

/// Generate shell completion scripts
///
/// Outputs completion script for the specified shell to stdout.
/// Users can redirect this to their shell's completion directory.
///
/// # Examples
///
/// ```bash
/// # Bash
/// wasm-slim completions bash > /etc/bash_completion.d/wasm-slim
///
/// # Zsh
/// wasm-slim completions zsh > ~/.zfunc/_wasm-slim
///
/// # Fish
/// wasm-slim completions fish > ~/.config/fish/completions/wasm-slim.fish
/// ```
pub fn cmd_completions(shell: Shell) {
    // We need to re-create the command structure here since Cli is in main.rs
    // This uses clap's derive API to generate completions
    use clap::{Arg, ArgAction, Command};

    let mut cmd = Command::new("wasm-slim")
        .version(env!("CARGO_PKG_VERSION"))
        .about("WASM bundle size optimizer")
        .arg(
            Arg::new("no-emoji")
                .long("no-emoji")
                .help("Disable emoji output")
                .action(ArgAction::SetTrue)
                .global(true),
        )
        .subcommand(Command::new("build").about("Build and optimize WASM binary"))
        .subcommand(Command::new("analyze").about("Analyze WASM bundle or dependencies"))
        .subcommand(Command::new("init").about("Initialize wasm-slim configuration"))
        .subcommand(Command::new("compare").about("Compare two WASM builds"))
        .subcommand(Command::new("completions").about("Generate shell completions"));

    let bin_name = "wasm-slim".to_string();
    generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
}

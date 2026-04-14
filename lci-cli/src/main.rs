#![warn(unused_crate_dependencies)]

use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;

mod lint;

/// Logical code interface for Rust crates.
#[derive(Parser)]
struct Cli {
  #[command(subcommand)]
  command: Command,
}

#[derive(Subcommand)]
enum Command {
  /// List impl blocks across a codebase.
  ImplBlocks {
    /// Path to a directory of Rust source files.
    #[arg(default_value = ".")]
    dir: PathBuf,
  },
  /// List structs across a codebase.
  Structs {
    /// Path to a directory of Rust source files.
    #[arg(default_value = ".")]
    dir: PathBuf,
  },
  /// List functions across a codebase.
  Functions {
    /// Path to a directory of Rust source files.
    #[arg(default_value = ".")]
    dir: PathBuf,
  },
  /// Send a prompt file to Claude Haiku and print the JSON response.
  SendPrompt {
    /// Path to a file containing the prompt text.
    prompt_file: PathBuf,
  },
  /// Find qualified paths (e.g. std::fmt::Display) in function bodies.
  QualifiedPaths {
    /// Path to a directory of Rust source files.
    #[arg(default_value = ".")]
    dir: PathBuf,
  },
  /// Run lint checks against a codebase.
  Lint(lint::Lint),
}

fn main() -> anyhow::Result<()> {
  let cli = Cli::parse();
  match cli.command {
    Command::ImplBlocks { dir } => {
      let codebase = lci::Codebase::parse(&dir)?;
      for b in codebase.impl_blocks() {
        println!("{b}");
      }
    },
    Command::Structs { dir } => {
      let codebase = lci::Codebase::parse(&dir)?;
      for s in codebase.structs() {
        println!("{s}");
      }
    },
    Command::Functions { dir } => {
      let codebase = lci::Codebase::parse(&dir)?;
      for f in codebase.functions() {
        println!("{f}");
      }
    },
    Command::QualifiedPaths { dir } => {
      let codebase = lci::Codebase::parse(&dir)?;
      for f in codebase.functions() {
        let paths = f.qualified_paths();
        if !paths.is_empty() {
          println!("{f}");
          for p in &paths {
            println!("  {p}");
          }
        }
      }
    },
    Command::SendPrompt { prompt_file } => {
      let prompt = std::fs::read_to_string(&prompt_file)?;
      let response = lci::send_prompt(&prompt)?;
      println!("{response}");
    },
    Command::Lint(cmd) => cmd.execute()?,
  }
  Ok(())
}

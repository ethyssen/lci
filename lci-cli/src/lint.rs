use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use clap::ValueEnum;

#[derive(Clone, ValueEnum)]
enum Mechanism {
  CliStructComment,
}

/// Run lint checks against a codebase.
#[derive(Parser)]
pub(crate) struct Lint {
  /// Path to a directory of Rust source files.
  #[arg(default_value = ".")]
  dir: PathBuf,
  /// Lint mechanism to run. Omit to run all.
  #[arg(long)]
  mechanism: Option<Mechanism>,
  /// Detect only, don't apply fixes.
  #[arg(long)]
  dry: bool,
  /// Print LLM input/output/usage and wait for enter between each candidate.
  #[arg(long)]
  debug: bool,
}

impl Lint {
  pub(crate) fn execute(&self) -> Result<()> {
    let mechanisms = match self.mechanism {
      Some(ref m) => vec![m.clone()],
      None => vec![Mechanism::CliStructComment],
    };
    for mechanism in mechanisms {
      match mechanism {
        Mechanism::CliStructComment => cli_struct_comment(&self.dir)?,
      }
    }
    Ok(())
  }
}

fn cli_struct_comment(_dir: &PathBuf) -> Result<()> {
  anyhow::bail!("cli-struct-comment not yet implemented")
}

#![warn(unused_crate_dependencies)]

use std::path::Path;

use anyhow::Result;

mod send_prompt;
pub use send_prompt::PromptResponse;
pub use send_prompt::send_prompt;

mod function;
pub use function::Edit;
pub use function::Function;

mod impl_block;
pub use impl_block::ImplBlock;

mod struct_def;
pub use struct_def::Struct;
use tree_sitter::Tree;

struct ParsedFile {
  source: String,
  tree: Tree,
}

pub struct Codebase {
  files: Vec<ParsedFile>,
}

impl Codebase {
  pub fn parse(dir: impl AsRef<Path>) -> Result<Self> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into())?;
    let mut files = vec![];
    for entry in glob::glob(dir.as_ref().join("**/*.rs").to_str().unwrap())? {
      let path = entry?;
      let source = std::fs::read_to_string(&path)?;
      let tree = parser
        .parse(&source, None)
        .ok_or_else(|| anyhow::anyhow!("parse failed: {}", path.display()))?;
      files.push(ParsedFile { source, tree });
    }
    Ok(Self { files })
  }

  pub fn functions(&self) -> Vec<Function<'_>> {
    self.files.iter().flat_map(|f| function::extract(&f.source, &f.tree)).collect()
  }

  pub fn impl_blocks(&self) -> Vec<ImplBlock<'_>> {
    self.files.iter().flat_map(|f| impl_block::extract(&f.source, &f.tree)).collect()
  }

  pub fn structs(&self) -> Vec<Struct<'_>> {
    self.files.iter().flat_map(|f| struct_def::extract(&f.source, &f.tree)).collect()
  }
}

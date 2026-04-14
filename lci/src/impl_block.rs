use tree_sitter::Node;

use crate::function::Function;

pub struct ImplBlock<'a> {
  source: &'a str,
  node: Node<'a>,
}

impl<'a> ImplBlock<'a> {
  fn target_type(&self) -> &str {
    self
      .node
      .child_by_field_name("type")
      .and_then(|n| n.utf8_text(self.source.as_bytes()).ok())
      .unwrap_or("")
  }

  fn trait_name(&self) -> Option<&str> {
    self.node.child_by_field_name("trait").and_then(|n| n.utf8_text(self.source.as_bytes()).ok())
  }

  fn methods(&self) -> Vec<Function<'a>> {
    let mut methods = vec![];
    if let Some(body) = self.node.child_by_field_name("body") {
      let mut cursor = body.walk();
      for item in body.children(&mut cursor) {
        if item.kind() == "function_item" {
          methods.push(Function::new(self.source, item));
        }
      }
    }
    methods
  }
}

impl std::fmt::Display for ImplBlock<'_> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let trait_part = match self.trait_name() {
      Some(t) => format!("{t} for "),
      None => String::new(),
    };
    let methods = self.methods();
    let methods: Vec<&str> = methods.iter().map(|m| m.name()).collect();
    write!(f, "impl {}{} [{}]", trait_part, self.target_type(), methods.join(", "))
  }
}

pub(crate) fn extract<'a>(source: &'a str, tree: &'a tree_sitter::Tree) -> Vec<ImplBlock<'a>> {
  let mut blocks = vec![];
  let root = tree.root_node();
  let mut cursor = root.walk();
  for child in root.children(&mut cursor) {
    if child.kind() == "impl_item" {
      blocks.push(parse_impl_block(source, child));
    }
  }
  blocks
}

fn parse_impl_block<'a>(source: &'a str, node: Node<'a>) -> ImplBlock<'a> {
  ImplBlock { source, node }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn parse(src: &str) -> (tree_sitter::Tree, &str) {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
    let tree = parser.parse(src, None).unwrap();
    (tree, src)
  }

  #[test]
  fn single_impl_block() {
    let src = r#"
struct Foo;

impl Foo {
    fn bar(&self) {}
    fn baz(&mut self) {}
}
"#;
    let (tree, src) = parse(src);
    let blocks = extract(src, &tree);
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].target_type(), "Foo");
    assert_eq!(blocks[0].trait_name(), None);
  }

  #[test]
  fn split_impl_blocks() {
    let src = r#"
struct Foo;

impl Foo {
    fn alpha(&self) {}
}

impl Foo {
    fn beta(&self) {}
}
"#;
    let (tree, src) = parse(src);
    let blocks = extract(src, &tree);
    assert_eq!(blocks.len(), 2);
    assert!(blocks.iter().all(|b| b.target_type() == "Foo"));
  }

  #[test]
  fn trait_impl_separate_from_inherent() {
    let src = r#"
struct Foo;

impl Foo {
    fn own_method(&self) {}
}

impl Display for Foo {
    fn fmt(&self, f: &mut Formatter) -> Result {
        Ok(())
    }
}
"#;
    let (tree, src) = parse(src);
    let blocks = extract(src, &tree);
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].trait_name(), None);
    assert_eq!(blocks[1].trait_name(), Some("Display"));
  }
}

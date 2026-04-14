use tree_sitter::Node;

pub struct Struct<'a> {
  source: &'a str,
  node: Node<'a>,
}

impl<'a> Struct<'a> {
  pub fn name(&self) -> &str {
    self
      .node
      .child_by_field_name("name")
      .and_then(|n| n.utf8_text(self.source.as_bytes()).ok())
      .unwrap_or("")
  }

  pub fn fields(&self) -> Vec<&str> {
    let Some(body) = self.node.child_by_field_name("body")
    else {
      return vec![];
    };
    let mut fields = vec![];
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
      if child.kind() == "field_declaration"
        && let Some(name_node) = child.child_by_field_name("name")
        && let Ok(text) = name_node.utf8_text(self.source.as_bytes())
      {
        fields.push(text);
      }
    }
    fields
  }
}

impl std::fmt::Display for Struct<'_> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "struct {} [{}]", self.name(), self.fields().join(", "))
  }
}

pub(crate) fn extract<'a>(source: &'a str, tree: &'a tree_sitter::Tree) -> Vec<Struct<'a>> {
  let mut structs = vec![];
  let root = tree.root_node();
  let mut cursor = root.walk();
  for child in root.children(&mut cursor) {
    if child.kind() == "struct_item" {
      structs.push(Struct { source, node: child });
    }
  }
  structs
}

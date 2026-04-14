use std::ops::Range;

use regex::Regex;
use tree_sitter::Node;

pub struct Function<'a> {
  source: &'a str,
  node: Node<'a>,
}

impl<'a> Function<'a> {
  pub(crate) fn new(source: &'a str, node: Node<'a>) -> Self {
    Self { source, node }
  }

  pub fn name(&self) -> &str {
    self
      .node
      .child_by_field_name("name")
      .and_then(|n| n.utf8_text(self.source.as_bytes()).ok())
      .unwrap_or("")
  }

  pub fn code(&self) -> &str {
    &self.source[self.node.byte_range()]
  }

  /// The function signature as source text (everything before the body block).
  pub fn signature(&self) -> &str {
    if let Some(body) = self.node.child_by_field_name("body") {
      self.source[self.node.start_byte()..body.start_byte()].trim()
    }
    else {
      self.code()
    }
  }

  /// The function's visibility as source text (e.g. `pub`, `pub(crate)`, or empty).
  pub fn visibility(&self) -> &str {
    let mut cursor = self.node.walk();
    for child in self.node.children(&mut cursor) {
      if child.kind() == "visibility_modifier" {
        return child.utf8_text(self.source.as_bytes()).unwrap_or("");
      }
    }
    ""
  }

  /// Returns deduplicated, sorted qualified paths found in the function body
  /// (e.g. `std::fmt::Display`, `std::collections::HashMap`).
  pub fn qualified_paths(&self) -> Vec<String> {
    use std::collections::BTreeSet;
    let code = self.code();
    let re = Regex::new(r"\b[a-zA-Z_]\w*(?:::[a-zA-Z_]\w*)+").unwrap();
    let paths: BTreeSet<String> = re.find_iter(code).map(|m| m.as_str().to_string()).collect();
    paths.into_iter().collect()
  }

  /// Find 1-indexed line numbers (relative to the function) where `name` appears
  /// as a word after `after_line`. Stops at a shadow (`let {name}`).
  pub fn variable_usages(&self, name: &str, after_line: usize) -> Vec<usize> {
    let code = self.code();
    let lines: Vec<&str> = code.lines().collect();
    let word_re = Regex::new(&format!(r"\b{}\b", regex::escape(name))).unwrap();
    let shadow_re = Regex::new(&format!(r"^\s*let\s+(mut\s+)?{}\b", regex::escape(name))).unwrap();
    let mut usages = vec![];
    for (i, line) in lines.iter().enumerate() {
      let line_num = i + 1;
      if line_num <= after_line {
        continue;
      }
      if shadow_re.is_match(line) {
        break;
      }
      let trimmed = line.trim();
      if trimmed.starts_with("//") || trimmed.starts_with("///") {
        continue;
      }
      if word_re.is_match(line) {
        usages.push(line_num);
      }
    }
    usages
  }

  /// Return lines `from..to` (1-indexed, exclusive) of the function source.
  pub fn lines_between(&self, from: usize, to: usize) -> String {
    let code = self.code();
    let lines: Vec<&str> = code.lines().collect();
    let start = from.min(lines.len());
    let end = to.saturating_sub(1).min(lines.len());
    lines[start..end].join("\n")
  }

  pub fn line_content(&self, line: usize) -> &str {
    self.code().lines().nth(line - 1).unwrap_or("")
  }

  /// Byte ranges in the source of all identifiers matching `old` within this function.
  /// Ranges are absolute positions in the full source text.
  /// Apply replacements back-to-front to preserve byte offsets.
  pub fn rename_variable(&self, old: &str, new: &str) -> Vec<Edit> {
    let mut ranges = vec![];
    collect_identifier_ranges(self.node, self.source, old, &mut ranges);
    ranges.sort_by(|a, b| b.start.cmp(&a.start));
    ranges.into_iter().map(|r| Edit { range: r, text: new.to_string() }).collect()
  }
}

pub struct Edit {
  pub range: Range<usize>,
  pub text: String,
}

fn collect_identifier_ranges(node: Node, source: &str, name: &str, ranges: &mut Vec<Range<usize>>) {
  if node.kind() == "identifier" && node.utf8_text(source.as_bytes()).ok() == Some(name) {
    ranges.push(node.byte_range());
    return;
  }
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    collect_identifier_ranges(child, source, name, ranges);
  }
}

impl std::fmt::Display for Function<'_> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "fn {}", self.name())
  }
}

pub(crate) fn extract<'a>(source: &'a str, tree: &'a tree_sitter::Tree) -> Vec<Function<'a>> {
  let mut functions = vec![];
  collect_functions(source, tree.root_node(), &mut functions);
  functions
}

fn collect_functions<'a>(source: &'a str, node: Node<'a>, functions: &mut Vec<Function<'a>>) {
  let mut cursor = node.walk();
  for child in node.children(&mut cursor) {
    match child.kind() {
      "function_item" => {
        functions.push(Function::new(source, child));
      },
      "impl_item" =>
        if let Some(body) = child.child_by_field_name("body") {
          let mut inner = body.walk();
          for item in body.children(&mut inner) {
            if item.kind() == "function_item" {
              functions.push(Function::new(source, item));
            }
          }
        },
      _ => {
        collect_functions(source, child, functions);
      },
    }
  }
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
  fn variable_usages_basic() {
    let src = r#"
fn example() {
    let x = 1;
    let y = 2;
    println!("{}", x + y);
    let z = x;
}
"#;
    let (tree, src) = parse(src);
    let fns = extract(src, &tree);
    let usages = fns[0].variable_usages("x", 2);
    assert_eq!(usages, vec![4, 5]);
  }

  #[test]
  fn variable_usages_stops_at_shadow() {
    let src = r#"
fn example() {
    let x = 1;
    println!("{x}");
    let x = 2;
    println!("{x}");
}
"#;
    let (tree, src) = parse(src);
    let fns = extract(src, &tree);
    let usages = fns[0].variable_usages("x", 2);
    assert_eq!(usages, vec![3]);
  }

  #[test]
  fn variable_usages_skips_comments() {
    let src = r#"
fn example() {
    let x = 1;
    // x is important
    println!("{x}");
}
"#;
    let (tree, src) = parse(src);
    let fns = extract(src, &tree);
    let usages = fns[0].variable_usages("x", 2);
    assert_eq!(usages, vec![4]);
  }

  #[test]
  fn free_and_method_functions() {
    let src = r#"
fn top_level() -> i32 {
    42
}

struct Bar;

impl Bar {
    fn method(&self) {}
}
"#;
    let (tree, src) = parse(src);
    let fns = extract(src, &tree);
    assert_eq!(fns.len(), 2);
    assert_eq!(fns[0].name(), "top_level");
    assert!(fns[0].code().contains("fn top_level()"));
    assert!(fns[0].code().contains("42"));
    assert_eq!(fns[1].name(), "method");
  }

  #[test]
  fn rename_variable_basic() {
    let src = r#"
fn example() {
    let items = vec![1, 2, 3];
    let n = items.len();
    println!("{}", items[0]);
}
"#;
    let (tree, src) = parse(src);
    let fns = extract(src, &tree);
    let edits = fns[0].rename_variable("items", "candidates");
    assert_eq!(edits.len(), 3);
    // Apply edits (already sorted back-to-front)
    let mut result = src.to_string();
    for edit in &edits {
      result.replace_range(edit.range.clone(), &edit.text);
    }
    assert!(result.contains("let candidates = vec![1, 2, 3];"));
    assert!(result.contains("candidates.len()"));
    assert!(result.contains("candidates[0]"));
    assert!(!result.contains("items"));
  }

  #[test]
  fn rename_variable_no_match() {
    let src = "fn example() { let x = 1; }\n";
    let (tree, src) = parse(src);
    let fns = extract(src, &tree);
    let edits = fns[0].rename_variable("y", "z");
    assert!(edits.is_empty());
  }
}

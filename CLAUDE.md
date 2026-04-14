# LCI - Logical Code Interface

LCI is a query layer over Rust codebases. It wraps `syn`'s parsed AST in thin view types (`Function`, `ImplBlock`, `Struct`) that provide query methods for automated pattern recognition and resolution.

## Why

We have a collection of code quality patterns (see `~/projects/tasks/mrt_reviews.txt`) that detect and fix defects at scale. These patterns need to ask questions like "what methods exist on this struct?" or "does this function accept a PathBuf instead of impl AsRef<Path>?" -- questions that are tedious and error-prone to answer with grep or raw AST walking. LCI provides a clean programmatic interface for these queries.

The system has three layers:
1. **LCI** (this project) - logical interface to the code: parse, query, modify
2. **Pattern recognizers** - detect code quality issues using LCI queries + optional LLM calls
3. **Pattern resolvers** - fix detected issues, either automatically or with human approval

## Design Philosophy

**View models, not data models.** LCI types are thin wrappers around `syn`'s output. They hold the parsed AST and provide query methods over it — they don't duplicate or replace what syn already understands.

- **syn** does the hard parsing work once
- **LCI types** (Function, ImplBlock, Struct) hold the syn AST and expose query methods computed on demand
- **Source text** stays available for display and LLM prompts, but is not the source of truth for analysis

This means:
- No information loss — every detail syn parsed is available, we just haven't written the query for it yet
- No duplication — no parallel data model that can drift from what syn understands
- Incremental buildout — add query methods as lints need them; the underlying data was always there

## Architecture

- **`lci`** crate: core library. `Codebase::parse(dir)` walks all `.rs` files and builds collections of `Struct`, `Function`, `ImplBlock` — each wrapping its corresponding syn type.
- **`lci-cli`** crate: CLI exposing LCI operations as subcommands.
- **`send_prompt`**: calls the Anthropic Messages API directly (not through Claude Code) using Haiku for cheap, fast LLM judgments on code patterns. Returns response text + token usage.

## Library API (`lci` crate)

### Codebase

`Codebase::parse(dir)` walks all `.rs` files and returns indexed collections.

- `functions() -> &[Function]`
- `structs() -> &[Struct]`
- `impl_blocks() -> &[ImplBlock]`

### Function

Wraps `syn::Signature` + `syn::Block` + `syn::Visibility`.

- `sig() -> &syn::Signature` — the parsed function signature
- `block() -> &syn::Block` — the parsed function body
- `vis() -> &syn::Visibility` — visibility
- `name() -> &str`
- `code() -> &str` — full source text (for display / LLM prompts)
- `file() -> &Path`
- `start_line() -> usize` — 1-indexed line in the file
- `let_bindings() -> Vec<LetBinding>` — top-level let statements, derived from `block().stmts`
- `variable_usages(name, after_line) -> Vec<usize>` — lines where `name` appears after `after_line`, stops at shadowing
- `qualified_paths() -> Vec<String>` — e.g. `std::fmt::Display`
- `lines_between(from, to) -> String` — extract code between two function-relative line numbers
- `line_content(line) -> &str` — single line by function-relative line number

### LetBinding

- `name: String` — variable name
- `expression: String` — RHS of the assignment
- `line: usize` — 1-indexed relative to the function
- `is_mut: bool`

### ImplBlock

Wraps `syn::ItemImpl`.

- `item() -> &syn::ItemImpl` — the parsed impl block
- `target_type() -> String` — the type being implemented
- `trait_name() -> Option<String>` — the trait, if this is a trait impl
- `methods() -> &[Function]`
- `file() -> &Path`
- `start_line() -> usize`

### Struct

Wraps `syn::ItemStruct`.

- `item() -> &syn::ItemStruct` — the parsed struct
- `name() -> &syn::Ident`
- `file() -> &Path`
- `start_line() -> usize`

### Other

- `send_prompt(prompt) -> Result<PromptResponse>` — calls Haiku via Anthropic API
- `PromptResponse` — `.text()`, `.usage()` (includes token counts and estimated cost)
- `move_line(path, from, to)` — moves a line from one position to another in a file (1-indexed), cleans up trailing blank lines

## Lints

Lint definitions live in `lints/<name>/` with:
- `prompt.txt` — LLM prompt template (use `{{CODE}}` for injection)
- `cases/` — test cases with `--- input ---` and `--- output ---` sections

## Conventions

- Rust only for now.
- Optimize for cost, speed, and complexity reduction.
- Do mechanical checks (line counting, usage counting, mutability) in LCI. Only send to the LLM what requires judgment (e.g. "is this move safe?").
- LLM calls use Haiku via direct API (`ANTHROPIC_API_KEY` env var required) to keep token costs minimal.
- Pattern detection should be high-confidence. When uncertain, ask the human.

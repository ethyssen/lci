use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

/// Send a prompt to Claude Haiku and return the response with usage info.
pub fn send_prompt(prompt: &str) -> Result<PromptResponse> {
  let api_key = std::env::var("ANTHROPIC_API_KEY")
    .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY env var not set"))?;

  let body = Request {
    model: "claude-haiku-4-5-20251001",
    max_tokens: 4096,
    messages: vec![Message { role: "user", content: prompt }],
  };

  let resp = reqwest::blocking::Client::new()
    .post("https://api.anthropic.com/v1/messages")
    .header("x-api-key", &api_key)
    .header("anthropic-version", "2023-06-01")
    .json(&body)
    .send()?;

  let status = resp.status();
  let raw = resp.text()?;
  if !status.is_success() {
    anyhow::bail!("Anthropic API error ({status}): {raw}");
  }

  let response: ApiResponse = serde_json::from_str(&raw)?;
  let text = response
    .content
    .into_iter()
    .next()
    .map(|b| b.text)
    .ok_or_else(|| anyhow::anyhow!("empty response from API"))?;
  let usage = Usage {
    input_tokens: response.usage.input_tokens,
    output_tokens: response.usage.output_tokens,
  };
  Ok(PromptResponse { text, usage })
}

pub struct PromptResponse {
  text: String,
  usage: Usage,
}

impl PromptResponse {
  pub fn text(&self) -> &str {
    &self.text
  }

  pub fn usage(&self) -> String {
    self.usage.to_string()
  }
}

impl std::fmt::Display for PromptResponse {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}\n\n{}", self.text, self.usage)
  }
}

struct Usage {
  input_tokens: u32,
  output_tokens: u32,
}

impl std::fmt::Display for Usage {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    // Haiku 4.5: $0.80/MTok input, $4.00/MTok output
    let cost =
      self.input_tokens as f64 * 0.0000008 + self.output_tokens as f64 * 0.000004;
    write!(
      f,
      "{} in / {} out / ${:.6}",
      self.input_tokens, self.output_tokens, cost
    )
  }
}

#[derive(Serialize)]
struct Request<'a> {
  model: &'a str,
  max_tokens: u32,
  messages: Vec<Message<'a>>,
}

#[derive(Serialize)]
struct Message<'a> {
  role: &'a str,
  content: &'a str,
}

#[derive(Deserialize)]
struct ApiResponse {
  content: Vec<ContentBlock>,
  usage: ApiUsage,
}

#[derive(Deserialize)]
struct ContentBlock {
  text: String,
}

#[derive(Deserialize)]
struct ApiUsage {
  input_tokens: u32,
  output_tokens: u32,
}

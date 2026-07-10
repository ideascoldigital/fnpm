//! Optional AI review of generated anti-corruption layers via a local
//! Ollama instance.
//!
//! Strictly advisory: it suggests domain-oriented naming and cohesion
//! improvements for the generated port/adapter, and it never blocks or
//! fails the `fnpm adapt` command. Nothing leaves the machine — the
//! request goes to a local HTTP endpoint (default `http://localhost:11434`).

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json::json;
use std::path::Path;
use std::time::Duration;

use crate::config::AiConfig;

/// A generated file to include in the review prompt.
pub struct ReviewFile<'a> {
    pub path: &'a Path,
    pub contents: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatMessage {
    content: String,
}

const SYSTEM_PROMPT: &str = "You are reviewing an anti-corruption layer (port + adapter) \
that was auto-generated from a JavaScript/TypeScript project's real usage of an npm package. \
The generated code mirrors the package API one-to-one; the goal of an anti-corruption layer \
is the opposite: an interface shaped by the project's domain, so the package's names and \
types do not leak into application code.\n\
Give short, concrete, actionable suggestions:\n\
- domain-oriented names for the port and its members (based on how they seem to be used)\n\
- members that could be merged, split, or hidden for better cohesion\n\
- package-specific types that should be replaced by the project's own types\n\
Reply with at most 8 bullet points, plain text, no code fences, no preamble.";

/// Build the user prompt from the package name and generated files.
fn build_prompt(package: &str, files: &[ReviewFile]) -> String {
    let mut prompt =
        format!("Package: '{package}'. Review this generated anti-corruption layer:\n\n");
    for file in files {
        prompt.push_str(&format!(
            "--- {} ---\n{}\n",
            file.path.display(),
            file.contents
        ));
    }
    prompt
}

/// Ask the configured local model to review the generated layer.
/// Returns the advisory text. Errors are meant to be reported as warnings
/// by the caller, never as command failures.
pub fn review_layer(ai: &AiConfig, package: &str, files: &[ReviewFile]) -> Result<String> {
    if ai.provider != "ollama" {
        return Err(anyhow!(
            "Unsupported AI provider '{}' (only 'ollama' is supported)",
            ai.provider
        ));
    }

    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(ai.timeout_seconds))
        .build()?;

    let url = format!("{}/api/chat", ai.url.trim_end_matches('/'));
    let body = json!({
        "model": ai.model,
        "stream": false,
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            { "role": "user", "content": build_prompt(package, files) },
        ],
    });

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .with_context(|| format!("Could not reach Ollama at {}", ai.url))?;

    let status = response.status();
    if !status.is_success() {
        let detail = response.text().unwrap_or_default();
        return Err(anyhow!(
            "Ollama returned {status}: {}",
            detail.chars().take(200).collect::<String>()
        ));
    }

    let chat: ChatResponse = response
        .json()
        .context("Unexpected response format from Ollama")?;

    let content = chat.message.content.trim().to_string();
    if content.is_empty() {
        return Err(anyhow!("Ollama returned an empty review"));
    }
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::thread;

    fn ai_config(url: String) -> AiConfig {
        AiConfig {
            enabled: true,
            provider: "ollama".to_string(),
            url,
            model: "test-model".to_string(),
            timeout_seconds: 5,
        }
    }

    fn review_files() -> Vec<(PathBuf, String)> {
        vec![(
            PathBuf::from("src/adapters/axios/axios.port.ts"),
            "export interface AxiosPort { get: typeof _default.get; }".to_string(),
        )]
    }

    fn as_review_files(files: &[(PathBuf, String)]) -> Vec<ReviewFile<'_>> {
        files
            .iter()
            .map(|(path, contents)| ReviewFile {
                path,
                contents: contents.clone(),
            })
            .collect()
    }

    /// Minimal one-shot HTTP server that answers with `body` and captures the request.
    fn spawn_server(status_line: &'static str, body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 65536];
                let mut total = 0;
                // Read until the JSON body is complete enough (headers + some body)
                loop {
                    match stream.read(&mut buf[total..]) {
                        Ok(0) => break,
                        Ok(n) => {
                            total += n;
                            let text = String::from_utf8_lossy(&buf[..total]);
                            if let Some(header_end) = text.find("\r\n\r\n") {
                                let content_length = text
                                    .lines()
                                    .find_map(|l| {
                                        l.to_ascii_lowercase()
                                            .strip_prefix("content-length:")
                                            .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                                    })
                                    .unwrap_or(0);
                                if total >= header_end + 4 + content_length {
                                    break;
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
                let response = format!(
                    "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        format!("http://{addr}")
    }

    #[test]
    fn test_build_prompt_includes_package_and_files() {
        let files = review_files();
        let prompt = build_prompt("axios", &as_review_files(&files));
        assert!(prompt.contains("'axios'"));
        assert!(prompt.contains("axios.port.ts"));
        assert!(prompt.contains("AxiosPort"));
    }

    #[test]
    fn test_review_layer_happy_path() {
        let url = spawn_server(
            "HTTP/1.1 200 OK",
            r#"{"message":{"role":"assistant","content":"- Rename AxiosPort to UserApiPort"}}"#,
        );
        let files = review_files();
        let review = review_layer(&ai_config(url), "axios", &as_review_files(&files)).unwrap();
        assert!(review.contains("UserApiPort"));
    }

    #[test]
    fn test_review_layer_server_error() {
        let url = spawn_server("HTTP/1.1 404 Not Found", r#"{"error":"model not found"}"#);
        let files = review_files();
        let err = review_layer(&ai_config(url), "axios", &as_review_files(&files)).unwrap_err();
        assert!(err.to_string().contains("404"));
    }

    #[test]
    fn test_review_layer_unreachable() {
        // Port from a listener we immediately drop: connection refused.
        let addr = {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.local_addr().unwrap()
        };
        let files = review_files();
        let err = review_layer(
            &ai_config(format!("http://{addr}")),
            "axios",
            &as_review_files(&files),
        )
        .unwrap_err();
        assert!(err.to_string().contains("Could not reach Ollama"));
    }

    #[test]
    fn test_review_layer_rejects_unknown_provider() {
        let mut ai = ai_config("http://localhost:1".to_string());
        ai.provider = "openai".to_string();
        let files = review_files();
        let err = review_layer(&ai, "axios", &as_review_files(&files)).unwrap_err();
        assert!(err.to_string().contains("Unsupported AI provider"));
    }

    #[test]
    fn test_review_layer_empty_content_is_error() {
        let url = spawn_server(
            "HTTP/1.1 200 OK",
            r#"{"message":{"role":"assistant","content":"   "}}"#,
        );
        let files = review_files();
        let err = review_layer(&ai_config(url), "axios", &as_review_files(&files)).unwrap_err();
        assert!(err.to_string().contains("empty review"));
    }
}

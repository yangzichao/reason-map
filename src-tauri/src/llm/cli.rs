//! LLM transport via the local Claude Code CLI (`claude -p`).
//!
//! The app uses the user's existing Claude Code login (subscription / OAuth) instead of an
//! Anthropic API key (SPEC §6, revised). We do NOT read or copy the OAuth token out of the
//! keychain — we invoke the official `claude` binary, which owns and manages that auth itself.
//! This is the supported headless / Agent-SDK path, not a credential-extraction hack.
//!
//! Interface mirrors the old HTTP client (`complete_text` / `stream_text`) so the high-level
//! ops in `llm/mod.rs` are unchanged. The user prompt is fed over stdin (no argv length /
//! escaping limits, safe for large maps); the system prompt + flags go on argv.

use std::process::Stdio;

use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

use crate::error::{AppError, AppResult};

const BIN: &str = "claude";
const MODEL: &str = "claude-opus-4-8";

/// Single-shot reasoning never needs tools; disabling them keeps each call to one model turn
/// and removes any chance of stalling on a permission prompt. Unknown names are harmless.
const DISABLED_TOOLS: &[&str] = &[
    "Bash", "Edit", "Write", "Read", "Task", "WebFetch", "WebSearch", "NotebookEdit", "Skill",
    "ToolSearch", "Workflow", "Glob", "Grep",
];

#[derive(Clone, Default)]
pub struct ClaudeCli;

impl ClaudeCli {
    pub fn new() -> Self {
        Self
    }

    /// Build the common `claude -p` invocation. `stream` switches between a single JSON result
    /// and a line-delimited stream-json event feed.
    fn command(system: &str, thinking: bool, stream: bool) -> Command {
        let mut cmd = Command::new(BIN);
        cmd.arg("-p")
            .arg("--model")
            .arg(MODEL)
            .arg("--system-prompt")
            .arg(system)
            .arg("--permission-mode")
            .arg("default");
        // Variadic flag: the tool names are consumed until the next `--flag`.
        cmd.arg("--disallowedTools");
        for t in DISABLED_TOOLS {
            cmd.arg(t);
        }
        // Map our `thinking` flag onto the CLI's effort dial (Opus 4.8 thinking is adaptive).
        if thinking {
            cmd.arg("--effort").arg("high");
        }
        if stream {
            cmd.arg("--output-format")
                .arg("stream-json")
                .arg("--include-partial-messages")
                .arg("--verbose");
        } else {
            cmd.arg("--output-format").arg("json");
        }
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cmd
    }

    /// Non-streaming completion. Returns the assistant text (the CLI's `result` field).
    pub async fn complete_text(
        &self,
        system: &str,
        user: &str,
        _max_tokens: u32,
        thinking: bool,
    ) -> AppResult<String> {
        let mut child = Self::command(system, thinking, false)
            .spawn()
            .map_err(spawn_error)?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(user.as_bytes())
                .await
                .map_err(|e| AppError::Llm(format!("write to claude stdin: {e}")))?;
            // Close stdin so the CLI knows the prompt is complete.
            let _ = stdin.shutdown().await;
        }

        let output = child
            .wait_with_output()
            .await
            .map_err(|e| AppError::Llm(format!("claude process: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            return Err(AppError::Llm(cli_failure_message(&output.status, &stderr)));
        }

        let parsed: CliResult = serde_json::from_str(stdout.trim()).map_err(|e| {
            AppError::Llm(format!("parse claude json: {e} — output: {}", stdout.trim()))
        })?;
        if parsed.is_error {
            return Err(AppError::Llm(
                parsed.result.unwrap_or_else(|| "claude reported an error".into()),
            ));
        }
        parsed
            .result
            .ok_or_else(|| AppError::Llm("claude returned no result".into()))
    }

    /// Streaming completion. Invokes `on_delta` for each text delta; returns the full text.
    pub async fn stream_text<F: FnMut(&str)>(
        &self,
        system: &str,
        user: &str,
        _max_tokens: u32,
        thinking: bool,
        mut on_delta: F,
    ) -> AppResult<String> {
        let mut child = Self::command(system, thinking, true)
            .spawn()
            .map_err(spawn_error)?;

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(user.as_bytes()).await;
            let _ = stdin.shutdown().await;
        }

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AppError::Llm("claude produced no stdout".into()))?;
        let mut lines = BufReader::new(stdout).lines();

        let mut full = String::new();
        let mut error_message: Option<String> = None;

        // Output is NDJSON: one event object per line. Text arrives as `content_block_delta`
        // events; the final `result` line carries the full text (used as a fallback) and the
        // error flag.
        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|e| AppError::Llm(format!("read claude stream: {e}")))?
        {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Ok(event) = serde_json::from_str::<StreamLine>(line) else {
                continue;
            };
            match event {
                StreamLine::StreamEvent { event } => {
                    if let StreamInner::ContentBlockDelta { delta } = event {
                        if let Some(text) = delta.text {
                            full.push_str(&text);
                            on_delta(&text);
                        }
                    }
                }
                StreamLine::Result { is_error, result } => {
                    if is_error {
                        error_message =
                            Some(result.unwrap_or_else(|| "claude reported an error".into()));
                    } else if full.is_empty() {
                        if let Some(r) = result {
                            full = r;
                        }
                    }
                }
                StreamLine::Other => {}
            }
        }

        let status = child
            .wait()
            .await
            .map_err(|e| AppError::Llm(format!("claude process: {e}")))?;

        if let Some(msg) = error_message {
            return Err(AppError::Llm(msg));
        }
        if !status.success() && full.is_empty() {
            let stderr = match child.stderr.take() {
                Some(mut s) => {
                    let mut buf = String::new();
                    let _ = tokio::io::AsyncReadExt::read_to_string(&mut s, &mut buf).await;
                    buf
                }
                None => String::new(),
            };
            return Err(AppError::Llm(cli_failure_message(&status, &stderr)));
        }
        Ok(full)
    }
}

/// Readiness of the local AI backend, surfaced to the UI so the user knows whether the
/// `claude` CLI is available (and reminded it must be logged in).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendStatus {
    pub ready: bool,
    pub version: Option<String>,
    pub detail: String,
}

/// Probe the CLI. `ready` means the binary is present and runnable; it does not by itself
/// prove the user is logged in (a real call surfaces any auth error). We deliberately do not
/// spend tokens to verify the session here.
pub async fn backend_status() -> BackendStatus {
    match Command::new(BIN).arg("--version").output().await {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
            BackendStatus {
                ready: true,
                version: Some(version),
                detail: "使用本机 Claude Code 登录态(订阅 / OAuth),无需 API key。若调用报未登录,运行 `claude login`。"
                    .into(),
            }
        }
        Ok(out) => BackendStatus {
            ready: false,
            version: None,
            detail: format!(
                "`claude --version` 执行失败:{}",
                String::from_utf8_lossy(&out.stderr).trim()
            ),
        },
        Err(_) => BackendStatus {
            ready: false,
            version: None,
            detail: "未找到 `claude` 命令。请先安装 Claude Code 并运行 `claude login` 登录你的订阅。".into(),
        },
    }
}

fn spawn_error(e: std::io::Error) -> AppError {
    if e.kind() == std::io::ErrorKind::NotFound {
        AppError::Llm("找不到 `claude` 命令 — 请安装 Claude Code 并运行 `claude login` 登录".into())
    } else {
        AppError::Llm(format!("启动 claude 失败:{e}"))
    }
}

fn cli_failure_message(status: &std::process::ExitStatus, stderr: &str) -> String {
    let stderr = stderr.trim();
    if stderr.is_empty() {
        format!("claude 退出码 {status}(可能未登录,试试 `claude login`)")
    } else {
        format!("claude 失败({status}):{stderr}")
    }
}

#[derive(Deserialize)]
struct CliResult {
    #[serde(default)]
    is_error: bool,
    #[serde(default)]
    result: Option<String>,
}

/// One line of `--output-format stream-json`. Tagged by the top-level `type`.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StreamLine {
    StreamEvent {
        event: StreamInner,
    },
    Result {
        #[serde(default)]
        is_error: bool,
        #[serde(default)]
        result: Option<String>,
    },
    #[serde(other)]
    Other,
}

/// The wrapped raw Anthropic stream event. We only care about text deltas.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StreamInner {
    ContentBlockDelta { delta: DeltaText },
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct DeltaText {
    /// Present for `text_delta`; absent for thinking / tool deltas (which we skip).
    #[serde(default)]
    text: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real end-to-end calls against the local `claude` CLI (needs Claude Code installed +
    // logged in; spends a few subscription tokens). Ignored by default so the normal
    // `cargo test` stays offline. Run explicitly:
    //   cargo test --lib llm::cli::tests -- --ignored --nocapture
    const SYS: &str = "You are a JSON API. Output only valid JSON, no prose, no tools.";

    #[tokio::test]
    #[ignore = "hits the live claude CLI / subscription"]
    async fn complete_text_roundtrip() {
        let out = ClaudeCli::new()
            .complete_text(SYS, "Return exactly: {\"ok\":true}", 200, false)
            .await
            .expect("complete_text failed");
        assert!(out.contains("\"ok\""), "unexpected result: {out}");
    }

    #[tokio::test]
    #[ignore = "hits the live claude CLI / subscription"]
    async fn stream_text_roundtrip() {
        let mut deltas = 0usize;
        let full = ClaudeCli::new()
            .stream_text(SYS, "Return exactly: {\"ok\":true}", 200, false, |_| deltas += 1)
            .await
            .expect("stream_text failed");
        assert!(full.contains("\"ok\""), "unexpected full text: {full}");
        assert!(deltas > 0, "expected at least one streamed delta");
    }
}

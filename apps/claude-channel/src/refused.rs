// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A start that cannot work is served, not exited. Claude Code starts the
//! bridge once per session and never again (SPIKE-001 fact 19), so a
//! bridge that exited on a usage error, a missing `XDG_RUNTIME_DIR` or a
//! bad profile name left the session with a server that "failed to
//! connect" and no reason anywhere the user looks. Refused, it stays a
//! functioning MCP server that says why.
//!
//! **It is never mistaken for a channel.** It declares no
//! `claude/channel` capability, so Claude Code registers no channel; it
//! holds no session and no lease; `status` reports the lease state
//! `bridge refused` with the refusal's class; and every other tool answers
//! the refusal as an error rather than a result (`tests/refused_start.rs`).

use interweave_claude_channel_core::Delivery;
use serde_json::{Value, json};
use tokio::io::{AsyncBufRead, AsyncWrite, AsyncWriteExt as _};

use crate::host_input::{HostLine, HostLines, MAX_HOST_LINE_BYTES};
use crate::mcp::{
    Incoming, METHOD_NOT_FOUND, PROTOCOL_VERSION, error_line, parse_line, result_line, tool_list,
    tool_result_line,
};

/// Why a start cannot work: a class a host can tell apart
/// (`TOOL-SURFACE.md` §status) and the text that says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// `usage`, `home_unset`, `runtime_dir_unset`, `profile_name` or
    /// `paths`.
    pub class: &'static str,
    /// What was wrong, as the start found it.
    pub reason: String,
}

/// What every answer says the refusal is.
#[must_use]
pub fn refusal_text(reason: &str) -> String {
    format!(
        "The InterWeave bridge did not start: {reason}. It is serving this session without \
         a channel or a connection to the transport daemon. Fix the plugin's profile or \
         endpoint setting (claude plugin configure interweave@interweave), or the \
         environment Claude Code starts it in, then start a new session."
    )
}

/// Serve `input` with the refusal until it ends. `delivery` is the mode
/// the arguments named, when they named one: its tools are listed, so the
/// session sees the surface it expected, each answering the refusal. With
/// none, push's.
///
/// # Errors
/// Only an I/O error on `input` or `output`.
pub async fn serve_refused<R, W>(
    refusal: &Refusal,
    delivery: Option<Delivery>,
    input: R,
    mut output: W,
) -> std::io::Result<()>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let delivery = delivery.unwrap_or(Delivery::Push);
    let text = refusal_text(&refusal.reason);
    let mut lines = HostLines::new(input);
    while let Some(line) = lines.next_line().await? {
        let HostLine::Text(line) = line else {
            if matches!(line, HostLine::TooLong) {
                eprintln!(
                    "claude-channel: a host line over {MAX_HOST_LINE_BYTES} bytes was dropped"
                );
            }
            continue;
        };
        let Incoming::Request { id, method, params } = parse_line(&line) else {
            continue;
        };
        let answer = answer(&id, &method, params.as_ref(), delivery, refusal, &text);
        output.write_all(answer.as_bytes()).await?;
        output.write_all(b"\n").await?;
        output.flush().await?;
    }
    Ok(())
}

fn answer(
    id: &Value,
    method: &str,
    params: Option<&Value>,
    delivery: Delivery,
    refusal: &Refusal,
    text: &str,
) -> String {
    match method {
        "initialize" => result_line(
            id,
            &json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "interweave", "version": env!("CARGO_PKG_VERSION")},
                "instructions": text,
            }),
        ),
        "ping" => result_line(id, &json!({})),
        "tools/list" => result_line(id, &json!({"tools": tool_list(delivery)})),
        // `status` is state, and a refused start is a state: answered as
        // a result. Every other tool would act, and cannot: an error.
        "tools/call"
            if params.and_then(|p| p.get("name")).and_then(Value::as_str) == Some("status") =>
        {
            let status = json!({
                "endpoint_lease_state": "bridge refused",
                "refused": {"class": refusal.class, "reason": refusal.reason},
            });
            tool_result_line(id, &status.to_string(), false)
        }
        "tools/call" => tool_result_line(id, text, true),
        _ => error_line(id, METHOD_NOT_FOUND, "method not found"),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn lines(input: &str, delivery: Option<Delivery>) -> Vec<Value> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("rt");
        let mut out = Vec::new();
        let refusal = Refusal {
            class: "runtime_dir_unset",
            reason: "XDG_RUNTIME_DIR is unset".to_owned(),
        };
        rt.block_on(serve_refused(
            &refusal,
            delivery,
            input.as_bytes(),
            &mut out,
        ))
        .expect("served");
        String::from_utf8(out)
            .expect("utf8")
            .lines()
            .map(|l| serde_json::from_str(l).expect("json"))
            .collect()
    }

    #[test]
    fn a_refused_start_declares_no_channel_and_says_why() {
        let out = lines(
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\n",
            Some(Delivery::Push),
        );
        let result = &out[0]["result"];
        assert!(
            result["capabilities"].get("experimental").is_none(),
            "no claude/channel: {result}"
        );
        assert!(
            result["instructions"]
                .as_str()
                .expect("text")
                .contains("XDG_RUNTIME_DIR is unset")
        );
    }

    #[test]
    fn every_tool_answers_the_refusal_as_an_error() {
        let out = lines(
            concat!(
                "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}\n",
                "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n",
                "not json\n",
                "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"send\"}}\n",
                "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"server/discover\"}\n",
                "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"tools/call\",\"params\":{\"name\":\"status\"}}\n",
            ),
            None,
        );
        assert_eq!(
            out.len(),
            4,
            "a notification and a malformed line are answered with nothing"
        );
        let listed = out[0]["result"]["tools"].as_array().expect("tools").len();
        assert_eq!(
            listed,
            tool_list(Delivery::Push).len(),
            "push's surface when no mode was named"
        );
        assert_eq!(out[1]["result"]["isError"], true);
        assert!(
            out[1]["result"]["content"][0]["text"]
                .as_str()
                .expect("text")
                .contains("did not start")
        );
        assert_eq!(
            out[2]["error"]["code"], METHOD_NOT_FOUND,
            "the era stays legacy"
        );
        assert_eq!(
            out[3]["result"]["isError"], false,
            "status is state, not an error"
        );
        let status: Value = serde_json::from_str(
            out[3]["result"]["content"][0]["text"]
                .as_str()
                .expect("text"),
        )
        .expect("json");
        assert_eq!(status["endpoint_lease_state"], "bridge refused");
        assert_eq!(status["refused"]["class"], "runtime_dir_unset");
    }
}

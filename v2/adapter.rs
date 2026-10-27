//! Compiled in by extensions/mod.rs, alongside (not inside) v2_sdk.
use super::v2_sdk;
use crate::native_core::common::{CoreContext, CoreResult};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    OnceLock,
};

#[path = "src/lib.rs"]
mod toon_core;

pub fn commands() -> &'static [&'static str] {
    &["convert-json"]
}

// The only global state is an opaque process nonce and monotonic token allocator.
// No session/source/output is kept globally. Cloning/discarding session state cannot
// roll this allocator back and revive a discarded action ID.
fn token() -> String {
    static NONCE: OnceLock<String> = OnceLock::new();
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let n = NEXT
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .expect("native TOON action counter exhausted");
    format!(
        "toon:{}:{n}",
        NONCE.get_or_init(|| uuid::Uuid::new_v4().to_string())
    )
}

fn preference_is(payload: &Value, name: &str, default: &str, expected: &str) -> bool {
    // Absent fields receive manifest defaults. Present corrupt/unknown values keep
    // the source's strict === semantics (non-copy => paste; non-auto => clipboard).
    payload
        .get("preferences")
        .and_then(|v| v.get(name))
        .map_or(default == expected, |v| v.as_str() == Some(expected))
}
fn acknowledged(reply: &Value) -> bool {
    reply.is_object()
        && reply.get("ok") == Some(&Value::Bool(true))
        && reply.get("error").is_none_or(Value::is_null)
        && reply.get("success").is_none_or(|v| v == &Value::Bool(true))
}
fn surface(state: &Value) -> Value {
    let phase = state["phase"].as_str().unwrap_or("error");
    let paste = state["action"] == "paste";
    let (message, title) = match phase {
        "intent" => ("Delivery is unconfirmed and may already have occurred. Retry only if you accept a duplicate copy or paste. This session retains the same converted bytes; it will not read your selection or clipboard again.", "Retry delivery (may repeat the effect)"),
        "done" => (if paste { "Converted! TOON format pasted to active app" } else { "Converted! TOON format copied to clipboard" }, ""),
        "empty" => ("No JSON found - selection and clipboard are empty", ""),
        _ => (state["message"].as_str().unwrap_or("Conversion Failed. Reopen the command to read a new source."), ""),
    };
    if phase != "intent" {
        return v2_sdk::no_view(message);
    }
    let actions = vec![json!({"id":state["token"],"title":title,"kind":"action"})];
    // Static status text only: no source-derived Markdown or built-in code-copy control.
    let mut view = v2_sdk::view(
        "JSON to TOON Converter",
        json!({"type":"Detail","props":{"markdown":message},"children":[]}),
        actions,
    );
    view["message"] = json!(message);
    view
}
fn failure(state: &mut Value, message: &str) -> Value {
    *state = json!({"phase":"error","message":message});
    surface(state)
}
async fn prepare(ctx: &CoreContext, payload: &Value, state: &mut Value) -> CoreResult {
    if state.get("phase").is_some() {
        return Err("This conversion session has already started.".into());
    }
    // JSON source declares no arguments. Ignore argumentValues and formValues.
    let mut text = String::new();
    if preference_is(payload, "inputSource", "auto", "auto") {
        if let Ok(value) = ctx.request("core:platform:selected-text", vec![]).await {
            let Some(selected) = value.as_str() else {
                return Ok(failure(
                    state,
                    "Conversion Failed. Selected-text service returned an invalid response.",
                ));
            };
            if selected.len() > toon_core::MAX_INPUT {
                return Ok(failure(state, toon_core::Error::InputLimit.message()));
            }
            text = selected.to_owned();
        }
        // Selected-text transport errors, as in V1, fall back to clipboard.
    }
    if toon_core::trim_empty(&text) {
        match ctx.request("clipboard:read", vec![]).await {
            Ok(value) => {
                let Some(clipboard) = value.as_str() else {
                    return Ok(failure(
                        state,
                        "Conversion Failed. Clipboard service returned an invalid response.",
                    ));
                };
                if clipboard.len() > toon_core::MAX_INPUT {
                    return Ok(failure(state, toon_core::Error::InputLimit.message()));
                }
                text = clipboard.to_owned();
            }
            Err(_) => {
                return Ok(failure(
                    state,
                    "Conversion Failed. Clipboard text could not be read.",
                ))
            }
        }
    }
    if toon_core::trim_empty(&text) {
        *state = json!({"phase":"empty"});
        return Ok(surface(state));
    }
    let output = match toon_core::convert(&text) {
        Ok(output) => output,
        Err(error) => return Ok(failure(state, error.message())),
    };
    let action = if preference_is(payload, "defaultAction", "copy", "copy") {
        "copy"
    } else {
        "paste"
    };
    *state = json!({"action":action,"output":output});
    deliver(ctx, state).await
}
// Launch state is private until a retry view commits. NativeSessions may discard it
// on cancellation even after router dispatch; no recovery or non-delivery is promised.
async fn deliver(ctx: &CoreContext, state: &mut Value) -> CoreResult {
    let output = state["output"]
        .as_str()
        .ok_or("Converted bytes are unavailable.")?
        .to_owned();
    let paste = state["action"] == "paste";
    // Checkpoint before await for both automatic launch and explicit retry.
    // Only a committed retry session retains this intent after cancellation.
    state["phase"] = json!("intent");
    state["token"] = json!(token());
    let reply = if paste {
        ctx.request("core:platform:paste-text", vec![json!(output)])
            .await
    } else {
        v2_sdk::copy(ctx, &output).await
    };
    if reply.as_ref().is_ok_and(acknowledged) {
        *state = json!({"phase":"done","action":if paste { "paste" } else { "copy" }});
    }
    // All transport/negative/ambiguous replies leave intent; never log their
    // potentially private messages and never automatically repeat an effect.
    Ok(surface(state))
}

pub async fn handle(
    ctx: &CoreContext,
    command: &str,
    operation: &str,
    payload: &Value,
    state: &mut Value,
) -> CoreResult {
    if command != "convert-json" {
        return Err("Unknown JSON to TOON command.".into());
    }
    match operation {
        "extension:run-command" => prepare(ctx, payload, state).await,
        "extension:refresh-session" | "extension:search-text-changed" | "extension:load-more" => {
            Ok(surface(state))
        }
        "extension:dispose-session" => {
            *state = json!({});
            Ok(v2_sdk::no_view(
                "Conversion session closed. Any unconfirmed delivery may already have occurred.",
            ))
        }
        "extension:invoke-action" => {
            if state["phase"] != "intent"
                || payload["actionId"].as_str().is_none()
                || payload["actionId"].as_str() != state["token"].as_str()
            {
                return Err(
                    "This delivery action is stale or does not belong to this session.".into(),
                );
            }
            deliver(ctx, state).await
        }
        _ => Err("Unsupported JSON to TOON operation.".into()),
    }
}

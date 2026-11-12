//! Ethone Studio MCP server: lets an AI assistant drive the suite's engines over stdio (JSON-RPC 2.0).

#[allow(dead_code)]
#[path = "../../pixel/src/doc.rs"]
mod doc;
#[allow(dead_code)]
#[path = "../../pixel/src/filters.rs"]
mod filters;
#[allow(dead_code)]
#[path = "../../pixel/src/inpaint.rs"]
mod inpaint;
#[allow(dead_code)]
#[path = "../../vector/src/model.rs"]
mod vector_model;
#[allow(dead_code)]
#[path = "../../lumen/src/develop.rs"]
mod develop;
#[allow(dead_code)]
#[path = "../../motion/src/model.rs"]
mod motion_model;
#[allow(dead_code)]
#[path = "../../page/src/model.rs"]
mod page_model;
#[allow(dead_code)]
#[path = "../../sound/src/audio.rs"]
mod audio;
mod tools;

use serde_json::{Value, json};
use std::io::{BufRead, Write};

const PROTOCOL: &str = "2025-06-18";

fn reply(out: &mut impl Write, id: &Value, result: Result<Value, (i64, String)>) {
    let msg = match result {
        Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
        Err((code, m)) => json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": m } }),
    };
    let _ = writeln!(out, "{msg}");
    let _ = out.flush();
}

fn handle(method: &str, params: &Value) -> Result<Value, (i64, String)> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": params["protocolVersion"].as_str().unwrap_or(PROTOCOL),
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "ethone-studio", "title": "Ethone Studio", "version": env!("CARGO_PKG_VERSION") },
            "instructions": "Outils de la suite Ethone Studio. Chemins de fichiers absolus. Chaque outil enregistre aussi un projet ouvrable dans l'app correspondante ; open_in_app l'ouvre pour l'utilisateur."
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools::list() })),
        "tools/call" => {
            let name = params["name"].as_str().unwrap_or_default().to_string();
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            let run = std::panic::catch_unwind(|| tools::call(&name, &args));
            Ok(match run {
                Ok(Ok(content)) => json!({ "content": content, "isError": false }),
                Ok(Err(e)) => json!({ "content": [{ "type": "text", "text": e }], "isError": true }),
                Err(_) => json!({ "content": [{ "type": "text", "text": format!("{name} : erreur interne") }], "isError": true }),
            })
        }
        _ => Err((-32601, format!("méthode inconnue : {method}"))),
    }
}

fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim_start_matches('\u{feff}');
        if line.trim().is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                reply(&mut out, &Value::Null, Err((-32700, e.to_string())));
                continue;
            }
        };
        let Some(id) = msg.get("id").cloned() else { continue };
        let method = msg["method"].as_str().unwrap_or_default();
        reply(&mut out, &id, handle(method, &msg["params"]));
    }
}

#[cfg(test)]
mod protocol_tests {
    use super::*;

    #[test]
    fn handshake_and_listing() {
        let init = handle("initialize", &json!({ "protocolVersion": "2025-03-26" })).unwrap();
        assert_eq!(init["protocolVersion"], "2025-03-26");
        let list = handle("tools/list", &json!({})).unwrap();
        let names: Vec<&str> = list["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
        for n in ["pixel_edit", "vector_draw", "lumen_develop", "motion_render", "page_pdf", "sound_process", "open_in_app"] {
            assert!(names.contains(&n), "{n}");
        }
        assert!(handle("nope", &json!({})).is_err());
        let bad = handle("tools/call", &json!({ "name": "pixel_edit", "arguments": {} })).unwrap();
        assert_eq!(bad["isError"], true);
    }
}

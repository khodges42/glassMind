use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use anyhow::Result;

use crate::config::Config;
use crate::context::ContextBundle;
use crate::db::IndexStore;
use crate::embedding::backend_from_config;

pub fn serve(config: &Config) -> Result<()> {
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr)?;
    println!("Glassmind listening on http://{addr}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(err) = handle_connection(config, stream) {
                    eprintln!("request failed: {err}");
                }
            }
            Err(err) => eprintln!("connection failed: {err}"),
        }
    }
    Ok(())
}

fn handle_connection(config: &Config, mut stream: TcpStream) -> Result<()> {
    let mut buffer = [0; 8192];
    let read = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..read]);
    let first_line = request.lines().next().unwrap_or_default();

    let response = if first_line.starts_with("GET /health ") {
        json_response(200, r#"{"status":"ok"}"#)
    } else if first_line.starts_with("GET /stats ") {
        let store = IndexStore::open(&config.vault.path.join(&config.database.path))?;
        json_response(200, &serde_json::to_string(&store.stats()?)?)
    } else if first_line.starts_with("POST /search ") {
        let body = request.split("\r\n\r\n").nth(1).unwrap_or_default();
        let query = json_field(body, "query").unwrap_or_default();
        let limit = json_field(body, "limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(10);
        let store = IndexStore::open(&config.vault.path.join(&config.database.path))?;
        let backend = backend_from_config(config);
        let hits = store.hybrid_search(&query, limit, backend.as_ref(), config)?;
        json_response(200, &serde_json::to_string(&hits)?)
    } else if first_line.starts_with("POST /context ") {
        let body = request.split("\r\n\r\n").nth(1).unwrap_or_default();
        let query = json_field(body, "query").unwrap_or_default();
        let limit = json_field(body, "limit")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(8);
        let budget = json_field(body, "budget")
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(6000);
        let store = IndexStore::open(&config.vault.path.join(&config.database.path))?;
        let backend = backend_from_config(config);
        let hits = store.hybrid_search(&query, limit, backend.as_ref(), config)?;
        let bundle = ContextBundle::from_hits(&query, budget, hits);
        json_response(200, &serde_json::to_string(&bundle)?)
    } else if first_line.starts_with("GET /notes/") {
        let raw_path = first_line
            .trim_start_matches("GET /notes/")
            .split_whitespace()
            .next()
            .unwrap_or_default();
        let note_path = config.vault.path.join(raw_path.replace("%20", " "));
        let content = std::fs::read_to_string(note_path)?;
        json_response(
            200,
            &serde_json::to_string(&serde_json::json!({ "content": content }))?,
        )
    } else {
        json_response(404, r#"{"error":"not found"}"#)
    };

    stream.write_all(response.as_bytes())?;
    Ok(())
}

fn json_response(status: u16, body: &str) -> String {
    let label = match status {
        200 => "OK",
        404 => "Not Found",
        _ => "OK",
    };
    format!(
        "HTTP/1.1 {status} {label}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    )
}

fn json_field(body: &str, field: &str) -> Option<String> {
    let value = serde_json::from_str::<serde_json::Value>(body).ok()?;
    value.get(field).map(|raw| {
        raw.as_str()
            .map(ToString::to_string)
            .unwrap_or_else(|| raw.to_string())
            .trim_matches('"')
            .to_string()
    })
}

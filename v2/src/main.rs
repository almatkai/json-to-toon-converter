use std::io::{self, BufRead};
fn main() {
    // NDJSON protocol only for reproducible offline tests. Production calls lib.rs directly.
    for line in io::stdin().lock().lines() {
        let result = line
            .ok()
            .and_then(|s| serde_json::from_str::<String>(&s).ok());
        let response = match result {
            Some(text) => match tezbar_json_to_toon_native::convert(&text) {
                Ok(output) => serde_json::json!({"ok":true,"output":output}),
                Err(error) => serde_json::json!({"ok":false,"error":error.message()}),
            },
            None => {
                serde_json::json!({"ok":false,"error":"Expected a JSON string containing source JSON."})
            }
        };
        println!("{response}");
    }
}

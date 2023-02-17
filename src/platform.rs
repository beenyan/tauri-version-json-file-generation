use serde_json::{json, Value};

pub fn new(signature: &str, url: &str) -> Value {
    json!({ "signature": signature, "url": url })
}

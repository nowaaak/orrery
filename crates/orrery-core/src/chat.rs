use serde_json::Value;

pub fn plain_text(value: &Value) -> String {
    strip_formatting(&component_text(value))
}

fn component_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts.iter().map(component_text).collect(),
        Value::Object(object) => {
            let mut text = object
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if let Some(Value::Array(extra)) = object.get("extra") {
                text.extend(extra.iter().map(component_text));
            }
            text
        }
        _ => String::new(),
    }
}

pub fn strip_formatting(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '§' {
            chars.next();
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flattens_components_and_strips_codes() {
        let value = serde_json::json!([
            {"text": "§aGreen ", "extra": [{"text": "and "}, "plain"]},
            " end",
            42
        ]);
        assert_eq!(plain_text(&value), "Green and plain end");
        assert_eq!(strip_formatting("§lBold§r text§"), "Bold text");
    }
}

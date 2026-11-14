//! A small JSON Schema validator for task outputs.
//!
//! The grammar already forces well-formed JSON of the right shape; this checks what a grammar
//! cannot (`maxItems`, string length, numeric bounds) and gives a readable error for a retry.
//! Supported keywords: `type`, `enum`, `const`, `properties`, `required`, `items`, `minItems`,
//! `maxItems`, `minLength`, `maxLength`, `minimum`, `maximum`, `anyOf`, `oneOf`.
//! Anything else is ignored.

use serde_json::Value;

/// Check `value` against `schema`. The error names the failing path, e.g. `$.skills[2]: ...`.
pub fn validate(schema: &Value, value: &Value) -> Result<(), String> {
    check(schema, value, "$")
}

fn type_ok(name: &str, v: &Value) -> bool {
    match name {
        "string" => v.is_string(),
        "integer" => v.is_i64() || v.is_u64() || v.as_f64().is_some_and(|f| f.fract() == 0.0),
        "number" => v.is_number(),
        "boolean" => v.is_boolean(),
        "null" => v.is_null(),
        "array" => v.is_array(),
        "object" => v.is_object(),
        _ => true,
    }
}

fn check(schema: &Value, v: &Value, path: &str) -> Result<(), String> {
    let Some(s) = schema.as_object() else {
        return Ok(());
    };
    if let Some(t) = s.get("type") {
        let names: Vec<&str> = match t {
            Value::String(n) => vec![n.as_str()],
            Value::Array(a) => a.iter().filter_map(Value::as_str).collect(),
            _ => vec![],
        };
        if !names.is_empty() && !names.iter().any(|n| type_ok(n, v)) {
            return Err(format!("{path}: expected {}", names.join(" or ")));
        }
    }
    if let Some(Value::Array(options)) = s.get("enum") {
        if !options.contains(v) {
            return Err(format!("{path}: not one of the allowed values"));
        }
    }
    if let Some(c) = s.get("const") {
        if c != v {
            return Err(format!("{path}: must equal {c}"));
        }
    }
    if let Some(Value::Array(alts)) = s.get("anyOf") {
        if !alts.iter().any(|a| check(a, v, path).is_ok()) {
            return Err(format!("{path}: matches none of the allowed shapes"));
        }
    }
    if let Some(Value::Array(alts)) = s.get("oneOf") {
        let n = alts.iter().filter(|a| check(a, v, path).is_ok()).count();
        if n != 1 {
            return Err(format!("{path}: must match exactly one shape, matches {n}"));
        }
    }
    if let Some(text) = v.as_str() {
        let len = text.chars().count() as u64;
        if let Some(min) = s.get("minLength").and_then(Value::as_u64) {
            if len < min {
                return Err(format!("{path}: shorter than {min} characters"));
            }
        }
        if let Some(max) = s.get("maxLength").and_then(Value::as_u64) {
            if len > max {
                return Err(format!("{path}: longer than {max} characters"));
            }
        }
    }
    if let Some(n) = v.as_f64() {
        if let Some(min) = s.get("minimum").and_then(Value::as_f64) {
            if n < min {
                return Err(format!("{path}: below {min}"));
            }
        }
        if let Some(max) = s.get("maximum").and_then(Value::as_f64) {
            if n > max {
                return Err(format!("{path}: above {max}"));
            }
        }
    }
    if let Some(items) = v.as_array() {
        if let Some(min) = s.get("minItems").and_then(Value::as_u64) {
            if (items.len() as u64) < min {
                return Err(format!("{path}: fewer than {min} items"));
            }
        }
        if let Some(max) = s.get("maxItems").and_then(Value::as_u64) {
            if items.len() as u64 > max {
                return Err(format!("{path}: more than {max} items"));
            }
        }
        if let Some(item_schema) = s.get("items") {
            for (i, item) in items.iter().enumerate() {
                check(item_schema, item, &format!("{path}[{i}]"))?;
            }
        }
    }
    if let Some(obj) = v.as_object() {
        if let Some(Value::Array(req)) = s.get("required") {
            for key in req.iter().filter_map(Value::as_str) {
                if !obj.contains_key(key) {
                    return Err(format!("{path}: missing required field \"{key}\""));
                }
            }
        }
        if let Some(Value::Object(props)) = s.get("properties") {
            for (key, sub) in props {
                if let Some(val) = obj.get(key) {
                    check(sub, val, &format!("{path}.{key}"))?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn accepts_and_rejects() {
        let schema = json!({
            "type": "object",
            "required": ["title", "skills"],
            "properties": {
                "title": {"type": "string", "minLength": 1},
                "level": {"enum": ["junior", "senior"]},
                "skills": {"type": "array", "items": {"type": "string"}, "maxItems": 2},
                "salary": {"type": ["integer", "null"], "minimum": 0}
            }
        });
        assert!(validate(
            &schema,
            &json!({"title": "Dev", "skills": ["a"], "salary": null})
        )
        .is_ok());
        let err = |v: Value| validate(&schema, &v).unwrap_err();
        assert!(err(json!({"skills": []})).contains("missing required field \"title\""));
        assert!(err(json!({"title": "", "skills": []})).contains("shorter"));
        assert!(err(json!({"title": "x", "skills": ["a", "b", "c"]})).contains("more than 2"));
        assert!(err(json!({"title": "x", "skills": [1]})).contains("$.skills[0]: expected string"));
        assert!(err(json!({"title": "x", "skills": [], "level": "lead"})).contains("allowed"));
        assert!(err(json!({"title": "x", "skills": [], "salary": -1})).contains("below"));
        assert!(err(json!({"title": "x", "skills": [], "salary": 1.5})).contains("expected"));
    }

    #[test]
    fn any_of_and_one_of() {
        let s = json!({"anyOf": [{"type": "string"}, {"type": "integer"}]});
        assert!(validate(&s, &json!("a")).is_ok());
        assert!(validate(&s, &json!(true)).is_err());
        let o = json!({"oneOf": [{"type": "number"}, {"type": "integer"}]});
        assert!(validate(&o, &json!(1)).is_err()); // matches both
        assert!(validate(&o, &json!(1.5)).is_ok());
    }
}

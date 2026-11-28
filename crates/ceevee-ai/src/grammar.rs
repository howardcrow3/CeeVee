//! JSON Schema to GBNF, so a local model can only produce output that parses
//! (TECHNICAL_PLAN section 5, "Tasks").
//!
//! Supported: `type` (string, integer, number, boolean, null, array, object, or a list of
//! these), `enum`, `const`, `properties` with `required`, `items` with `minItems`, `anyOf`,
//! `oneOf`, single-element `allOf`, and `$ref` into `$defs` / `definitions`. Objects emit only
//! their declared properties, in schema order. `maxItems`, string patterns and numeric bounds are
//! not enforced by the grammar, so validate those after parsing.

use std::collections::BTreeMap;

use ceevee_core::{Error, Result};
use serde_json::Value;

const PRIMITIVES: &str = r#"ws ::= | " " | "\n" [ \t]{0,20}
string ::= "\"" ( [^"\\\x7F\x00-\x1F] | "\\" (["\\/bfnrt] | "u" [0-9a-fA-F]{4}) )* "\"" ws
integer ::= "-"? ([0-9] | [1-9] [0-9]{0,15}) ws
number ::= "-"? ([0-9] | [1-9] [0-9]{0,15}) ("." [0-9]+)? ([eE] [-+]? [0-9]+)? ws
boolean ::= ("true" | "false") ws
null ::= "null" ws
"#;

/// Compile a JSON Schema document into a GBNF grammar whose start rule is `root`.
pub fn json_schema_to_gbnf(schema: &Value) -> Result<String> {
    let mut c = Compiler {
        root: schema,
        rules: BTreeMap::new(),
        counter: 0,
    };
    let body = c.expr(schema, "root")?;
    c.rules.insert("root".into(), body);
    let mut out = String::new();
    for (name, body) in &c.rules {
        out.push_str(&format!("{name} ::= {body}\n"));
    }
    out.push_str(PRIMITIVES);
    Ok(out)
}

struct Compiler<'a> {
    root: &'a Value,
    rules: BTreeMap<String, String>,
    counter: usize,
}

fn unsupported(what: impl std::fmt::Display) -> Error {
    Error::Invalid(format!(
        "schema not supported by the grammar compiler: {what}"
    ))
}

/// A GBNF string literal for the exact characters of `s`.
fn lit(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A GBNF literal matching the JSON encoding of `v` (strings quoted and escaped).
fn json_lit(v: &Value) -> String {
    lit(&v.to_string())
}

fn rule_name(def: &str) -> String {
    let cleaned: String = def
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    format!("def-{cleaned}")
}

impl<'a> Compiler<'a> {
    fn fresh(&mut self, hint: &str) -> String {
        self.counter += 1;
        format!("{hint}-{}", self.counter)
    }

    /// Returns a GBNF expression for `schema`. Complex pieces become their own rules.
    fn expr(&mut self, schema: &'a Value, hint: &str) -> Result<String> {
        let Some(obj) = schema.as_object() else {
            return Err(unsupported("a schema must be an object"));
        };
        if let Some(r) = obj.get("$ref").and_then(Value::as_str) {
            return self.reference(r);
        }
        if let Some(c) = obj.get("const") {
            return Ok(format!("{} ws", json_lit(c)));
        }
        if let Some(Value::Array(values)) = obj.get("enum") {
            let alts: Vec<String> = values.iter().map(json_lit).collect();
            return Ok(format!("({}) ws", alts.join(" | ")));
        }
        for key in ["anyOf", "oneOf"] {
            if let Some(Value::Array(options)) = obj.get(key) {
                return self.alternatives(options, hint);
            }
        }
        if let Some(Value::Array(all)) = obj.get("allOf") {
            return match all.as_slice() {
                [only] => self.expr(only, hint),
                _ => Err(unsupported("allOf with several parts")),
            };
        }
        match obj.get("type") {
            Some(Value::String(t)) => self.typed(t, obj, hint),
            Some(Value::Array(types)) => {
                let mut alts = Vec::new();
                for t in types {
                    let t = t.as_str().ok_or_else(|| unsupported("non-string type"))?;
                    alts.push(self.typed(t, obj, hint)?);
                }
                Ok(format!("({})", alts.join(" | ")))
            }
            _ => Err(unsupported(
                "a schema needs type, enum, const, anyOf, oneOf or $ref",
            )),
        }
    }

    fn alternatives(&mut self, options: &'a [Value], hint: &str) -> Result<String> {
        let mut alts = Vec::new();
        for o in options {
            alts.push(self.expr(o, hint)?);
        }
        Ok(format!("({})", alts.join(" | ")))
    }

    fn reference(&mut self, r: &str) -> Result<String> {
        let Some(pointer) = r.strip_prefix('#') else {
            return Err(unsupported(format!("external $ref {r}")));
        };
        let name = rule_name(pointer.rsplit('/').next().unwrap_or(pointer));
        if !self.rules.contains_key(&name) {
            // Reserve the name first so recursive types terminate.
            self.rules.insert(name.clone(), String::new());
            let target = self
                .root
                .pointer(pointer)
                .ok_or_else(|| unsupported(format!("unresolved $ref {r}")))?;
            let body = self.expr(target, &name)?;
            self.rules.insert(name.clone(), body);
        }
        Ok(name)
    }

    fn typed(
        &mut self,
        t: &str,
        obj: &'a serde_json::Map<String, Value>,
        hint: &str,
    ) -> Result<String> {
        match t {
            "string" => Ok("string".into()),
            "integer" => Ok("integer".into()),
            "number" => Ok("number".into()),
            "boolean" => Ok("boolean".into()),
            "null" => Ok("null".into()),
            "array" => {
                let items = obj
                    .get("items")
                    .ok_or_else(|| unsupported("array without items"))?;
                let item = self.expr(items, hint)?;
                let item_rule = self.fresh(&format!("{hint}-item"));
                self.rules.insert(item_rule.clone(), item);
                let min = obj.get("minItems").and_then(Value::as_u64).unwrap_or(0);
                let list = format!("{item_rule} (\",\" ws {item_rule})*");
                Ok(if min >= 1 {
                    format!("\"[\" ws {list} \"]\" ws")
                } else {
                    format!("\"[\" ws ({list})? \"]\" ws")
                })
            }
            "object" => self.object(obj, hint),
            other => Err(unsupported(format!("type {other}"))),
        }
    }

    fn object(&mut self, obj: &'a serde_json::Map<String, Value>, hint: &str) -> Result<String> {
        let props: Vec<(&'a String, &'a Value)> = obj
            .get("properties")
            .and_then(Value::as_object)
            .map(|m| m.iter().collect())
            .unwrap_or_default();
        let required: Vec<&str> = obj
            .get("required")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        for r in &required {
            if !props.iter().any(|(k, _)| k.as_str() == *r) {
                return Err(unsupported(format!("required property {r} has no schema")));
            }
        }
        // Each property becomes `"key" ws ":" ws <value>`.
        let mut req = Vec::new();
        let mut opt = Vec::new();
        for (key, schema) in props {
            let value = self.expr(schema, hint)?;
            let kv = format!(
                "{} ws \":\" ws {value}",
                lit(&format!(
                    "\"{}\"",
                    key.replace('\\', "\\\\").replace('"', "\\\"")
                ))
            );
            if required.contains(&key.as_str()) {
                req.push(kv);
            } else {
                opt.push(kv);
            }
        }
        let comma = "\",\" ws ";
        let body = if !req.is_empty() {
            let mut s = req.join(&format!(" {comma}"));
            for kv in &opt {
                s.push_str(&format!(" ({comma}{kv})?"));
            }
            s
        } else if opt.is_empty() {
            String::new()
        } else {
            // No required property: the object may start at any optional one.
            let mut alts = Vec::new();
            for i in 0..opt.len() {
                let mut s = opt[i].clone();
                for kv in &opt[i + 1..] {
                    s.push_str(&format!(" ({comma}{kv})?"));
                }
                alts.push(s);
            }
            alts.push(String::new());
            format!("({})", alts.join(" | "))
        };
        Ok(format!("\"{{\" ws {body} \"}}\" ws"))
    }
}

#[cfg(test)]
mod tests;

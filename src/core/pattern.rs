//! Pattern: Pike's structural regexp for scrolls (x/g/v/then)

use anyhow::{anyhow, Result};
use nine_s_core::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

/// Raw pattern definition (for serialization)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternDef {
    pub name: String,
    pub watch: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub x: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub g: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub v: Option<String>,
    pub emit: String,
    pub emit_path: String,
    pub template: Value,
    #[serde(skip_serializing_if = "Option::is_none")] pub then: Option<String>,
}

/// Compiled pattern with cached regexes
#[derive(Debug, Clone)]
pub struct Pattern {
    pub name: String,
    pub watch: String,
    watch_pattern: WatchPattern,
    x: Option<Arc<Regex>>,
    g: Option<Arc<Regex>>,
    v: Option<Arc<Regex>>,
    pub emit: String,
    pub emit_path: String,
    pub template: Value,
    pub then: Option<String>,
}

impl Pattern {
    pub fn compile(def: PatternDef) -> Result<Self> {
        let watch_pattern = WatchPattern::parse(&def.watch)
            .map_err(|e| anyhow!("invalid watch pattern '{}': {}", def.watch, e))?;
        let compile_re = |s: &Option<String>| s.as_ref().map(|s| Regex::new(s)).transpose().map(|r| r.map(Arc::new));
        Ok(Self {
            name: def.name, watch: def.watch, watch_pattern,
            x: compile_re(&def.x)?, g: compile_re(&def.g)?, v: compile_re(&def.v)?,
            emit: def.emit, emit_path: def.emit_path, template: def.template, then: def.then,
        })
    }

    pub fn from_value(value: Value) -> Result<Self> { Self::compile(serde_json::from_value(value)?) }
}

impl Pattern {
    pub fn matches_path(&self, path: &str) -> bool { self.watch_pattern.matches(path) }

    pub fn apply(&self, scroll: &Scroll, origin: Option<&str>) -> Result<Option<Scroll>> {
        if !self.matches_path(&scroll.key) { return Ok(None); }
        let data_str = serde_json::to_string(&scroll.data)?;
        if self.g.as_ref().map(|g| !g.is_match(&data_str)).unwrap_or(false) { return Ok(None); }
        if self.v.as_ref().map(|v| v.is_match(&data_str)).unwrap_or(false) { return Ok(None); }

        let captures: Vec<String> = self.x.as_ref()
            .and_then(|x| x.captures(&data_str))
            .map(|c| c.iter().skip(1).filter_map(|m| m.map(|m| m.as_str().into())).collect())
            .unwrap_or_default();
        let segs: Vec<&str> = scroll.key.split('/').filter(|s| !s.is_empty()).collect();

        let metadata = origin.map(|o| Metadata::default().with_produced_by(o)).unwrap_or_default();
        Ok(Some(Scroll {
            key: substitute(&self.emit_path, &captures, &segs, &scroll.data),
            type_: self.emit.clone(),
            metadata,
            data: substitute_value(&self.template, &captures, &segs, &scroll.data),
        }))
    }
}


fn substitute(template: &str, caps: &[String], segs: &[&str], data: &Value) -> String {
    let mut r = template.to_string();
    for (i, c) in caps.iter().enumerate() { r = r.replace(&format!("${{{}}}", i + 1), c); }
    for (i, s) in segs.iter().enumerate() { r = r.replace(&format!("${{path.{}}}", i), s); }
    r = r.replace("${uuid}", &short_id());
    if let Value::Object(obj) = data {
        for (k, v) in obj {
            let val = match v { Value::String(s) => s.clone(), Value::Number(n) => n.to_string(), _ => continue };
            r = r.replace(&format!("${{data.{}}}", k), &val);
        }
    }
    r
}

fn substitute_value(tpl: &Value, caps: &[String], segs: &[&str], data: &Value) -> Value {
    match tpl {
        Value::String(s) => Value::String(substitute(s, caps, segs, data)),
        Value::Object(obj) => Value::Object(obj.iter()
            .map(|(k, v)| (substitute(k, caps, segs, data), substitute_value(v, caps, segs, data)))
            .collect()),
        Value::Array(arr) => Value::Array(arr.iter().map(|v| substitute_value(v, caps, segs, data)).collect()),
        other => other.clone(),
    }
}

fn short_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    format!("{:08x}", (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() & 0xFFFFFFFF) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_watch_pattern_matching() {
        // Uses WatchPattern from nine-s-core
        let pattern = WatchPattern::parse("/push/*/pending/*").unwrap();
        assert!(pattern.matches("/push/abc/pending/123"));
        assert!(!pattern.matches("/push/abc/def/pending/123"));

        let deep = WatchPattern::parse("/push/**/pending/*").unwrap();
        assert!(deep.matches("/push/abc/def/pending/123"));

        let sys = WatchPattern::parse("/sys/**").unwrap();
        assert!(sys.matches("/sys/mind/patterns/test"));
    }

    #[test]
    fn test_pattern_apply() {
        let def = PatternDef {
            name: "test".to_string(),
            watch: "/push/*/pending/*".to_string(),
            x: Some(r#""event":"(\w+)""#.to_string()),
            g: Some(r#""event":"payment""#.to_string()),
            v: None,
            emit: "external/apns@v1".to_string(),
            emit_path: "/external/apns/${path.1}/${uuid}".to_string(),
            template: json!({
                "alert": "Payment received!",
                "user": "${path.1}"
            }),
            then: None,
        };
        let pattern = Pattern::compile(def).unwrap();

        let scroll = Scroll {
            key: "/push/abc123/pending/pay-001".to_string(),
            type_: "push/webhook@v1".to_string(),
            metadata: Metadata::default(),
            data: json!({
                "event": "payment",
                "user_hint": "abc123"
            }),
        };

        let result = pattern.apply(&scroll, None).unwrap();
        assert!(result.is_some());

        let reaction = result.unwrap();
        assert!(reaction.key.starts_with("/external/apns/abc123/"));
        assert_eq!(reaction.data["user"], "abc123");
    }
}

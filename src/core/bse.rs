//! BSE: Block Structural Expressions
//!
//! Pike's Structural Regular Expressions adapted for UI block rendering.
//! Reusable across WASM, native, and any framework.
//!
//! ## The 8 Operators
//!
//! Core (extraction & filtering):
//! - `x` - Extract: match and collect blocks
//! - `y` - Between: collect gaps between matches
//! - `g` - Guard: keep if predicate true
//! - `v` - Void: keep if predicate false
//!
//! Output:
//! - `c` - Change: transform to rendered output
//!
//! Modifiers:
//! - `o` - Order: sort by field
//! - `n` - Take: limit results
//! - `l` - Layout: wrap in container
//!
//! ## DSL Syntax
//!
//! ```text
//! x/type=hero/ c/HeroBlock/
//! x/type=post/ g/published/ o/date,desc/ n/5/ l/stack,16/ c/PostCard/
//! ```

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;

/// A BSE pipeline stage
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum Stage {
    /// x/pattern/ - Extract matching blocks
    X { pattern: Predicate },
    /// y/pattern/ - Extract gaps between matches (inverse of x)
    Y { pattern: Predicate },
    /// g/predicate/ - Guard: keep if true
    G { predicate: Predicate },
    /// v/predicate/ - Void: keep if false
    V { predicate: Predicate },
    /// c/renderer/ - Change: transform to output
    C { renderer: String, #[serde(default)] props: Value },
    /// o/field,dir/ - Order: sort results
    O { field: String, #[serde(default)] desc: bool },
    /// n/count/ - Take: limit results
    N { count: usize },
    /// l/mode/ { children } - Layout: wrap in container
    L { mode: LayoutMode, #[serde(default)] gap: Option<u32>, children: Pipeline },
}

/// Layout modes
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LayoutMode {
    #[default]
    Stack,
    Row,
    Grid { cols: u32 },
    Absolute,
    None,
}

/// A predicate for filtering blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Predicate {
    /// Field path (supports dot notation: "meta.author")
    pub field: String,
    /// Comparison operator
    #[serde(default)]
    pub op: PredicateOp,
    /// Value to compare against (None = exists check)
    #[serde(default)]
    pub value: Option<Value>,
}

/// Predicate operators
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PredicateOp {
    #[default]
    Eq,
    Ne,
    Gt,
    Lt,
    Gte,
    Lte,
    Contains,
    Exists,
}

/// A BSE pipeline
pub type Pipeline = Vec<Stage>;

/// BSE output node (framework-agnostic virtual DOM)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BSENode {
    /// Renderer/component name (e.g., "HeroBlock", "PostCard")
    pub renderer: String,
    /// Props passed to renderer
    pub props: Value,
    /// Stable key for diffing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Child nodes (for layouts)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<BSENode>,
}

/// BSE evaluator
pub struct BSEEngine;

impl BSEEngine {
    /// Evaluate a pipeline against source blocks
    pub fn evaluate(pipeline: &Pipeline, source: &[Value]) -> Result<Vec<BSENode>> {
        let mut current: Vec<Value> = source.to_vec();

        for stage in pipeline {
            match stage {
                Stage::X { pattern } => {
                    current = current.into_iter()
                        .filter(|b| Self::matches(b, pattern))
                        .collect();
                }
                Stage::Y { pattern } => {
                    current = current.into_iter()
                        .filter(|b| !Self::matches(b, pattern))
                        .collect();
                }
                Stage::G { predicate } => {
                    current = current.into_iter()
                        .filter(|b| Self::matches(b, predicate))
                        .collect();
                }
                Stage::V { predicate } => {
                    current = current.into_iter()
                        .filter(|b| !Self::matches(b, predicate))
                        .collect();
                }
                Stage::O { field, desc } => {
                    current.sort_by(|a, b| {
                        let ord = Self::compare_field(a, b, field);
                        if *desc { ord.reverse() } else { ord }
                    });
                }
                Stage::N { count } => {
                    current.truncate(*count);
                }
                Stage::C { renderer, props } => {
                    return Ok(current.into_iter().map(|block| {
                        let key = Self::get_key(&block);
                        let mut merged_props = props.clone();
                        if let (Value::Object(m), Value::Object(b)) = (&mut merged_props, &block) {
                            for (k, v) in b {
                                m.insert(k.clone(), v.clone());
                            }
                        } else {
                            merged_props = block;
                        }
                        BSENode {
                            renderer: renderer.clone(),
                            props: merged_props,
                            key,
                            children: vec![],
                        }
                    }).collect());
                }
                Stage::L { mode, gap, children } => {
                    let child_nodes = Self::evaluate(children, &current)?;
                    return Ok(vec![BSENode {
                        renderer: Self::layout_renderer(mode),
                        props: serde_json::json!({
                            "mode": mode,
                            "gap": gap,
                        }),
                        key: None,
                        children: child_nodes,
                    }]);
                }
            }
        }

        Ok(vec![])
    }

    /// Check if a block matches a predicate
    fn matches(block: &Value, pred: &Predicate) -> bool {
        let field_value = Self::get_field(block, &pred.field);

        match (&pred.op, &pred.value, field_value) {
            // Exists check
            (PredicateOp::Exists, _, Some(_)) => true,
            (PredicateOp::Exists, _, None) => false,

            // Value comparisons
            (PredicateOp::Eq, Some(v), Some(fv)) => fv == v,
            (PredicateOp::Ne, Some(v), Some(fv)) => fv != v,

            // Numeric comparisons
            (PredicateOp::Gt, Some(v), Some(fv)) => Self::compare_values(fv, v) == Ordering::Greater,
            (PredicateOp::Lt, Some(v), Some(fv)) => Self::compare_values(fv, v) == Ordering::Less,
            (PredicateOp::Gte, Some(v), Some(fv)) => Self::compare_values(fv, v) != Ordering::Less,
            (PredicateOp::Lte, Some(v), Some(fv)) => Self::compare_values(fv, v) != Ordering::Greater,

            // Contains (string or array)
            (PredicateOp::Contains, Some(v), Some(Value::String(s))) => {
                if let Value::String(needle) = v {
                    s.contains(needle.as_str())
                } else {
                    false
                }
            }
            (PredicateOp::Contains, Some(v), Some(Value::Array(arr))) => {
                arr.contains(v)
            }

            // Default: no match
            _ => false,
        }
    }

    /// Get a nested field value (supports dot notation)
    fn get_field<'a>(block: &'a Value, path: &str) -> Option<&'a Value> {
        let mut current = block;
        for segment in path.split('.') {
            match current {
                Value::Object(obj) => {
                    current = obj.get(segment)?;
                }
                Value::Array(arr) => {
                    if let Ok(idx) = segment.parse::<usize>() {
                        current = arr.get(idx)?;
                    } else {
                        return None;
                    }
                }
                _ => return None,
            }
        }
        Some(current)
    }

    /// Get a stable key from a block (for VDOM diffing)
    fn get_key(block: &Value) -> Option<String> {
        block.get("id")
            .or_else(|| block.get("key"))
            .or_else(|| block.get("_id"))
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                Value::Number(n) => Some(n.to_string()),
                _ => None,
            })
    }

    /// Compare two values for sorting
    fn compare_values(a: &Value, b: &Value) -> Ordering {
        match (a, b) {
            (Value::Number(a), Value::Number(b)) => {
                a.as_f64().unwrap_or(0.0)
                    .partial_cmp(&b.as_f64().unwrap_or(0.0))
                    .unwrap_or(Ordering::Equal)
            }
            (Value::String(a), Value::String(b)) => a.cmp(b),
            (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
            _ => Ordering::Equal,
        }
    }

    /// Compare two blocks by a field
    fn compare_field(a: &Value, b: &Value, field: &str) -> Ordering {
        let va = Self::get_field(a, field);
        let vb = Self::get_field(b, field);
        match (va, vb) {
            (Some(va), Some(vb)) => Self::compare_values(va, vb),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    }

    /// Get renderer name for layout mode
    fn layout_renderer(mode: &LayoutMode) -> String {
        match mode {
            LayoutMode::Stack => "BSEStack".into(),
            LayoutMode::Row => "BSERow".into(),
            LayoutMode::Grid { .. } => "BSEGrid".into(),
            LayoutMode::Absolute => "BSEAbsolute".into(),
            LayoutMode::None => "BSEFragment".into(),
        }
    }
}

/// Parse BSE DSL to Pipeline
pub fn parse_dsl(input: &str) -> Result<Pipeline> {
    let mut pipeline = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\n' | '\t' => { chars.next(); }
            'x' | 'y' | 'g' | 'v' => {
                let op = chars.next().unwrap();
                expect_char(&mut chars, '/')?;
                let pattern_str = read_until(&mut chars, '/')?;
                let predicate = parse_predicate(&pattern_str)?;
                match op {
                    'x' => pipeline.push(Stage::X { pattern: predicate }),
                    'y' => pipeline.push(Stage::Y { pattern: predicate }),
                    'g' => pipeline.push(Stage::G { predicate }),
                    'v' => pipeline.push(Stage::V { predicate }),
                    _ => unreachable!(),
                }
            }
            'c' => {
                chars.next();
                expect_char(&mut chars, '/')?;
                let renderer = read_until(&mut chars, '/')?;
                pipeline.push(Stage::C { renderer, props: Value::Object(Default::default()) });
            }
            'o' => {
                chars.next();
                expect_char(&mut chars, '/')?;
                let order_str = read_until(&mut chars, '/')?;
                let parts: Vec<&str> = order_str.split(',').collect();
                let field = parts.first().ok_or_else(|| anyhow!("missing field in o//"))?.to_string();
                let desc = parts.get(1).map(|d| *d == "desc").unwrap_or(false);
                pipeline.push(Stage::O { field, desc });
            }
            'n' => {
                chars.next();
                expect_char(&mut chars, '/')?;
                let count_str = read_until(&mut chars, '/')?;
                let count = count_str.parse().map_err(|_| anyhow!("invalid count in n//"))?;
                pipeline.push(Stage::N { count });
            }
            'l' => {
                chars.next();
                expect_char(&mut chars, '/')?;
                let layout_str = read_until(&mut chars, '/')?;
                let (mode, gap) = parse_layout_mode(&layout_str)?;
                // Look for { children }
                skip_whitespace(&mut chars);
                let children = if chars.peek() == Some(&'{') {
                    chars.next(); // consume '{'
                    let inner = read_until_balanced(&mut chars, '{', '}')?;
                    parse_dsl(&inner)?
                } else {
                    Vec::new()
                };
                pipeline.push(Stage::L { mode, gap, children });
            }
            ';' => { chars.next(); }
            '{' => {
                chars.next();
                let inner = read_until_balanced(&mut chars, '{', '}')?;
                pipeline.extend(parse_dsl(&inner)?);
            }
            _ => return Err(anyhow!("unexpected character: {}", c)),
        }
    }

    Ok(pipeline)
}

fn expect_char(chars: &mut std::iter::Peekable<std::str::Chars>, expected: char) -> Result<()> {
    match chars.next() {
        Some(c) if c == expected => Ok(()),
        Some(c) => Err(anyhow!("expected '{}', got '{}'", expected, c)),
        None => Err(anyhow!("unexpected end of input, expected '{}'", expected)),
    }
}

fn read_until(chars: &mut std::iter::Peekable<std::str::Chars>, delimiter: char) -> Result<String> {
    let mut result = String::new();
    while let Some(&c) = chars.peek() {
        if c == delimiter {
            chars.next();
            return Ok(result);
        }
        result.push(chars.next().unwrap());
    }
    Err(anyhow!("unexpected end of input, expected '{}'", delimiter))
}

fn read_until_balanced(chars: &mut std::iter::Peekable<std::str::Chars>, open: char, close: char) -> Result<String> {
    let mut result = String::new();
    let mut depth = 1;
    while let Some(c) = chars.next() {
        if c == open { depth += 1; }
        if c == close {
            depth -= 1;
            if depth == 0 { return Ok(result); }
        }
        result.push(c);
    }
    Err(anyhow!("unbalanced braces"))
}

fn skip_whitespace(chars: &mut std::iter::Peekable<std::str::Chars>) {
    while matches!(chars.peek(), Some(' ' | '\n' | '\t')) {
        chars.next();
    }
}

fn parse_predicate(s: &str) -> Result<Predicate> {
    // Try different operators in order of specificity
    for (op_str, op) in [
        (">=", PredicateOp::Gte),
        ("<=", PredicateOp::Lte),
        ("!=", PredicateOp::Ne),
        ("=", PredicateOp::Eq),
        (">", PredicateOp::Gt),
        ("<", PredicateOp::Lt),
        ("~", PredicateOp::Contains),
    ] {
        if let Some(idx) = s.find(op_str) {
            let field = s[..idx].trim().to_string();
            let value_str = s[idx + op_str.len()..].trim();
            let value = parse_value(value_str);
            return Ok(Predicate { field, op, value: Some(value) });
        }
    }

    // Check for negation (existence check)
    if s.starts_with('!') {
        return Ok(Predicate {
            field: s[1..].trim().to_string(),
            op: PredicateOp::Exists,
            value: Some(Value::Bool(false)),
        });
    }

    // Plain field = exists check
    Ok(Predicate {
        field: s.trim().to_string(),
        op: PredicateOp::Exists,
        value: None,
    })
}

fn parse_value(s: &str) -> Value {
    // Try to parse as JSON first
    if let Ok(v) = serde_json::from_str(s) {
        return v;
    }
    // Boolean
    if s == "true" { return Value::Bool(true); }
    if s == "false" { return Value::Bool(false); }
    // Number
    if let Ok(n) = s.parse::<i64>() { return Value::Number(n.into()); }
    if let Ok(n) = s.parse::<f64>() { return serde_json::Number::from_f64(n).map(Value::Number).unwrap_or(Value::String(s.to_string())); }
    // String (unquoted)
    Value::String(s.to_string())
}

fn parse_layout_mode(s: &str) -> Result<(LayoutMode, Option<u32>)> {
    let parts: Vec<&str> = s.split(',').collect();
    let mode_str = parts.first().ok_or_else(|| anyhow!("missing layout mode"))?;
    let gap = parts.get(1).and_then(|g| g.parse().ok());

    let mode = match *mode_str {
        "stack" => LayoutMode::Stack,
        "row" => LayoutMode::Row,
        "absolute" => LayoutMode::Absolute,
        "none" => LayoutMode::None,
        s if s.starts_with("grid") => {
            // grid or grid,3
            let cols = parts.get(1)
                .and_then(|c| c.parse().ok())
                .or_else(|| s.strip_prefix("grid").and_then(|c| c.parse().ok()))
                .unwrap_or(1);
            return Ok((LayoutMode::Grid { cols }, parts.get(2).and_then(|g| g.parse().ok())));
        }
        _ => return Err(anyhow!("unknown layout mode: {}", mode_str)),
    };

    Ok((mode, gap))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_simple() {
        let pipeline = parse_dsl("x/type=hero/ c/HeroBlock/").unwrap();
        assert_eq!(pipeline.len(), 2);
    }

    #[test]
    fn test_parse_full_pipeline() {
        let pipeline = parse_dsl("x/type=post/ g/published/ o/date,desc/ n/5/ c/PostCard/").unwrap();
        assert_eq!(pipeline.len(), 5);
    }

    #[test]
    fn test_evaluate_simple() {
        let pipeline = parse_dsl("x/type=hero/ c/HeroBlock/").unwrap();
        let source = vec![
            json!({"type": "hero", "title": "Welcome"}),
            json!({"type": "post", "title": "Blog"}),
        ];
        let result = BSEEngine::evaluate(&pipeline, &source).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].renderer, "HeroBlock");
        assert_eq!(result[0].props["title"], "Welcome");
    }

    #[test]
    fn test_evaluate_with_order_and_take() {
        let pipeline = parse_dsl("x/type=post/ o/score,desc/ n/2/ c/PostCard/").unwrap();
        let source = vec![
            json!({"type": "post", "title": "A", "score": 10}),
            json!({"type": "post", "title": "B", "score": 30}),
            json!({"type": "post", "title": "C", "score": 20}),
        ];
        let result = BSEEngine::evaluate(&pipeline, &source).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].props["title"], "B"); // score 30
        assert_eq!(result[1].props["title"], "C"); // score 20
    }

    #[test]
    fn test_guard_and_void() {
        let pipeline = parse_dsl("x/type=post/ g/published=true/ v/draft/ c/PostCard/").unwrap();
        let source = vec![
            json!({"type": "post", "published": true, "title": "Good"}),
            json!({"type": "post", "published": false, "title": "Unpublished"}),
            json!({"type": "post", "published": true, "draft": true, "title": "Draft"}),
        ];
        let result = BSEEngine::evaluate(&pipeline, &source).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].props["title"], "Good");
    }

    #[test]
    fn test_nested_field_access() {
        let pipeline = parse_dsl("x/meta.author=sam/ c/AuthorCard/").unwrap();
        let source = vec![
            json!({"meta": {"author": "sam"}, "title": "Sam's post"}),
            json!({"meta": {"author": "other"}, "title": "Other's post"}),
        ];
        let result = BSEEngine::evaluate(&pipeline, &source).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].props["title"], "Sam's post");
    }

    #[test]
    fn test_y_between() {
        let pipeline = parse_dsl("y/type=hero/ c/Block/").unwrap();
        let source = vec![
            json!({"type": "hero", "title": "Hero"}),
            json!({"type": "post", "title": "Post"}),
        ];
        let result = BSEEngine::evaluate(&pipeline, &source).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].props["title"], "Post"); // Non-hero
    }
}

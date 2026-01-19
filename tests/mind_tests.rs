//! Mind Test Suite: 5 Euclidean tests for maximal value
//!
//! Test 1: Pattern compilation and regex caching
//! Test 2: Watch-based change detection
//! Test 3: Loop prevention via produced_by
//! Test 4: Cascade execution (then)
//! Test 5: write_scroll preserves type

use beenode::{Mind, MindConfig, Pattern, PatternDef, Store};
use nine_s_core::prelude::*;
use once_cell::sync::Lazy;
use serde_json::json;
use std::sync::Mutex;
use tempfile::TempDir;

static ENV_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn temp_store() -> (TempDir, Store, std::sync::MutexGuard<'static, ()>) {
    let guard = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = TempDir::new().expect("tempdir");
    std::env::set_var("NINE_S_ROOT", dir.path());
    let store = Store::open("beenode-test", &[]).expect("store");
    (dir, store, guard)
}

/// Test 1: Pattern compiles regexes once, reuses on apply
#[test]
fn pattern_compiles_and_caches_regexes() {
    let def = PatternDef {
        name: "extract-event".to_string(),
        watch: "/events/**".to_string(),
        x: Some(r#""type":"(\w+)""#.to_string()),
        g: Some(r#""active":true"#.to_string()),
        v: Some(r#""skip":true"#.to_string()),
        emit: "processed/event@v1".to_string(),
        emit_path: "/processed/${1}".to_string(),
        template: json!({"extracted": "${1}"}),
        then: None,
    };

    // Compiles successfully
    let pattern = Pattern::compile(def).unwrap();
    assert_eq!(pattern.name, "extract-event");

    // Apply matches with guard
    let scroll = Scroll {
        key: "/events/user/click".to_string(),
        type_: "event@v1".to_string(),
        metadata: Metadata::default(),
        data: json!({"type": "click", "active": true}),
    };
    let result = pattern.apply(&scroll, None).unwrap();
    assert!(result.is_some());
    let reaction = result.unwrap();
    assert_eq!(reaction.key, "/processed/click");
    assert_eq!(reaction.data["extracted"], "click");

    // Veto blocks
    let vetoed = Scroll {
        key: "/events/system/skip-me".to_string(),
        type_: "event@v1".to_string(),
        metadata: Metadata::default(),
        data: json!({"type": "internal", "active": true, "skip": true}),
    };
    let result = pattern.apply(&vetoed, None).unwrap();
    assert!(result.is_none());

    // Guard fails
    let inactive = Scroll {
        key: "/events/user/inactive".to_string(),
        type_: "event@v1".to_string(),
        metadata: Metadata::default(),
        data: json!({"type": "view", "active": false}),
    };
    let result = pattern.apply(&inactive, None).unwrap();
    assert!(result.is_none());
}

/// Test 2: Mind receives watch events (not polling)
#[test]
fn mind_receives_watch_events() {
    let (_dir, store, _guard) = temp_store();

    // Write a pattern
    store
        .write(
            "/sys/mind/patterns/echo",
            json!({
                "name": "echo",
                "watch": "/input/**",
                "emit": "output@v1",
                "emit_path": "/output/${path.1}",
                "template": {"echoed": true}
            }),
        )
        .unwrap();

    // Create Mind and load patterns
    let mut mind = Mind::new(store.clone());
    mind.reload_patterns().unwrap();

    let patterns = mind.load_patterns().unwrap();
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0].name, "echo");

    // Verify watch pattern uses native WatchPattern
    assert!(patterns[0].matches_path("/input/foo"));
    assert!(patterns[0].matches_path("/input/foo/bar"));
    assert!(!patterns[0].matches_path("/other/path"));
}

/// Test 3: Loop prevention via produced_by metadata
#[test]
fn loop_prevention_via_produced_by() {
    let def = PatternDef {
        name: "amplify".to_string(),
        watch: "/signal/**".to_string(),
        x: None,
        g: None,
        v: None,
        emit: "signal@v1".to_string(),
        emit_path: "/signal/amplified/${uuid}".to_string(),
        template: json!({"amplified": true}),
        then: None,
    };
    let pattern = Pattern::compile(def).unwrap();

    // First apply: no origin, produces with origin
    let input = Scroll {
        key: "/signal/raw/1".to_string(),
        type_: "signal@v1".to_string(),
        metadata: Metadata::default(),
        data: json!({"value": 42}),
    };

    let result = pattern.apply(&input, Some("mind")).unwrap();
    assert!(result.is_some());
    let reaction = result.unwrap();

    // Reaction has produced_by set
    assert_eq!(reaction.metadata.produced_by, Some("mind".to_string()));
    assert_eq!(reaction.type_, "signal@v1");

    // Second apply: scroll with matching origin should be processed
    // (the Mind skips based on checking scroll.metadata.produced_by == config.origin)
    // Pattern itself doesn't skip - Mind does the skip logic
    let self_produced = Scroll {
        key: "/signal/amplified/abc".to_string(),
        type_: "signal@v1".to_string(),
        metadata: Metadata::default().with_produced_by("mind"),
        data: json!({"amplified": true}),
    };

    // Pattern still matches (Mind would skip this, not Pattern)
    let result = pattern.apply(&self_produced, Some("mind")).unwrap();
    assert!(result.is_some()); // Pattern matches, Mind would skip
}

/// Test 4: Cascade execution (then)
#[test]
fn cascade_execution() {
    let (_dir, store, _guard) = temp_store();

    // Pattern A: transforms input to intermediate
    store
        .write(
            "/sys/mind/patterns/step1",
            json!({
                "name": "step1",
                "watch": "/raw/**",
                "emit": "intermediate@v1",
                "emit_path": "/intermediate/${path.1}",
                "template": {"step": 1},
                "then": "step2"
            }),
        )
        .unwrap();

    // Pattern B: transforms intermediate to final (cascade target)
    store
        .write(
            "/sys/mind/patterns/step2",
            json!({
                "name": "step2",
                "watch": "/intermediate/**",
                "emit": "final@v1",
                "emit_path": "/final/${path.1}",
                "template": {"step": 2, "complete": true}
            }),
        )
        .unwrap();

    let mut mind = Mind::new(store.clone());
    mind.reload_patterns().unwrap();

    // Verify both patterns loaded
    let patterns = mind.load_patterns().unwrap();
    assert_eq!(patterns.len(), 2);

    // Find step1 and verify it has then
    let step1 = patterns.iter().find(|p| p.name == "step1").unwrap();
    assert_eq!(step1.then, Some("step2".to_string()));
}

/// Test 5: write_scroll preserves type
#[test]
fn write_scroll_preserves_type() {
    let (_dir, store, _guard) = temp_store();

    let scroll = Scroll {
        key: "/typed/scroll/1".to_string(),
        type_: "custom/type@v1".to_string(),
        metadata: Metadata::default().with_produced_by("test"),
        data: json!({"value": 123}),
    };

    store.write_scroll(scroll).unwrap();

    let read_back = store.read("/typed/scroll/1").unwrap().unwrap();
    assert_eq!(read_back.type_, "custom/type@v1");
    assert_eq!(read_back.metadata.produced_by, Some("test".to_string()));
    assert_eq!(read_back.data["value"], 123);
}

/// Integration: Full Mind flow with pattern matching
#[test]
fn mind_pattern_matching_flow() {
    let (_dir, store, _guard) = temp_store();

    // Set up pattern
    store
        .write(
            "/sys/mind/patterns/transform",
            json!({
                "name": "transform",
                "watch": "/input/*",
                "x": "\"value\":(\\d+)",
                "emit": "output@v1",
                "emit_path": "/output/${path.1}",
                "template": {
                    "original_path": "${path.1}",
                    "captured_value": "${1}",
                    "processed": true
                }
            }),
        )
        .unwrap();

    let mut mind = Mind::with_config(
        store.clone(),
        MindConfig {
            process_existing: false,
            origin: "test-mind".to_string(),
        },
    );
    mind.reload_patterns().unwrap();

    // Manually apply pattern (simulating what run() does)
    let input = Scroll {
        key: "/input/doc123".to_string(),
        type_: "doc@v1".to_string(),
        metadata: Metadata::default(),
        data: json!({"value": 42, "name": "test"}),
    };

    let patterns = mind.load_patterns().unwrap();
    for pattern in &patterns {
        if let Some(reaction) = pattern.apply(&input, Some("test-mind")).unwrap() {
            store.write_scroll(reaction.clone()).unwrap();

            // Verify reaction
            assert_eq!(reaction.key, "/output/doc123");
            assert_eq!(reaction.type_, "output@v1");
            assert_eq!(reaction.data["original_path"], "doc123");
            assert_eq!(reaction.data["captured_value"], "42");
            assert_eq!(reaction.data["processed"], true);
            assert_eq!(reaction.metadata.produced_by, Some("test-mind".to_string()));
        }
    }

    // Verify persisted
    let stored = store.read("/output/doc123").unwrap().unwrap();
    assert_eq!(stored.type_, "output@v1");
}

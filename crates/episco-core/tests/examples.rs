//! The example files shipped in `examples/` must always load cleanly.

use std::path::Path;

use episco_core::{ewm, Service};

fn examples_dir() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples"))
}

#[test]
fn every_example_ewm_parses_without_warnings() {
    let dir = examples_dir().join("liturgies");
    let mut count = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some(ewm::EXTENSION) {
            continue;
        }
        let parsed = ewm::load_file(&path).unwrap();
        assert!(parsed.warnings.is_empty(), "{}: {:?}", path.display(), parsed.warnings);
        // No markup may leak through as literal text (e.g. `**` split across lines).
        for block in &parsed.document.blocks {
            assert!(!block.plain_text().contains("**"), "{}: literal `**` in {block:?}", path.display());
        }
        // Writer output must re-parse to the same document.
        let again = ewm::parse(&ewm::to_string(&parsed.document)).unwrap();
        assert_eq!(again.document, parsed.document, "{}", path.display());
        count += 1;
    }
    assert!(count >= 5);
}

#[test]
fn example_service_plan_loads() {
    let loaded = Service::load_plan(examples_dir().join("service.toml")).unwrap();
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    assert_eq!(loaded.service.cues.len(), 10);
}

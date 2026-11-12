//! The example files shipped in `examples/` must always load cleanly.

use std::path::Path;

use episco_core::{ewm, Service};

fn examples_dir() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples"))
}

#[test]
fn every_example_ewm_parses_without_warnings() {
    assert!(check_dir(&examples_dir().join("liturgies")) >= 5);
}

#[test]
fn every_builtin_library_item_parses_without_warnings() {
    assert!(check_dir(&examples_dir().join("../library")) >= 20);
}

/// Parses every `.ewm` in `dir`, requiring no warnings and a lossless round trip.
fn check_dir(dir: &Path) -> usize {
    let mut count = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
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
        assert!(!parsed.document.meta.title.is_empty(), "{}: no title", path.display());
        assert!(parsed.document.meta.category.is_some(), "{}: no type", path.display());
        count += 1;
    }
    count
}

#[test]
fn example_service_plan_loads() {
    let loaded = Service::load_plan(examples_dir().join("service.toml")).unwrap();
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    assert_eq!(loaded.service.cues.len(), 10);
}

/// The example service makes a bulletin and a script that zip up as Word
/// files. Set `EPISCO_DOCX_OUT` to a folder to keep them for a look.
#[test]
fn example_service_makes_word_files() {
    use episco_core::bulletin::{bulletin, script, ParishInfo};
    let mut service = Service::load_plan(examples_dir().join("service.toml")).unwrap().service;
    service.time = Some("10:30".into());
    // With a real Bible file, fill the readings as the app does.
    if let Ok(path) = std::env::var("EPISCO_BIBLE") {
        use episco_core::bible::{set_reading_passage, BibleText, BibleVersion};
        let bible = BibleText::from_json(&std::fs::read_to_string(path).unwrap()).unwrap();
        let ids: Vec<String> = service.cues.iter().map(|c| c.id.clone()).collect();
        for id in ids {
            let cue = service.cue(&id).unwrap();
            if !episco_core::bulletin::needs_text(cue) {
                continue;
            }
            let doc = cue.document().unwrap();
            let text = bible.passage(doc.meta.citation.as_deref().unwrap(), true).unwrap();
            let doc = set_reading_passage(doc, &text, BibleVersion::Nrsvue, 40);
            service.set_document(&id, doc).unwrap();
        }
        assert!(!service.cues.iter().any(episco_core::bulletin::needs_text));
    }
    let parish = ParishInfo {
        name: "St. Example's Episcopal Church".into(),
        address: "1 Church Street\nAnytown".into(),
        welcome: "We are glad you are here.".into(),
        ..Default::default()
    };
    for (name, doc) in [("bulletin", bulletin(&service, &parish)), ("script", script(&service, &parish))] {
        assert!(!doc.entries.is_empty());
        let bytes = episco_core::docx::to_docx(&doc).unwrap();
        assert!(bytes.starts_with(b"PK"));
        if let Ok(dir) = std::env::var("EPISCO_DOCX_OUT") {
            std::fs::write(Path::new(&dir).join(format!("{name}.docx")), &bytes).unwrap();
            std::fs::write(Path::new(&dir).join(format!("{name}.html")), episco_core::bulletin::to_html(&doc)).unwrap();
        }
    }
}

/// Every reading of the lectionary for three years of Sundays and feasts can
/// be found in a Bible file. Needs a real file (not shipped: copyright), so
/// it runs only with `EPISCO_BIBLE=/path/to/NRSVue.json`.
#[test]
fn every_lectionary_reading_is_in_the_bible_file() {
    use episco_core::{bible::BibleText, calendar::Date, lectionary, propers::ProperKind};
    let Ok(path) = std::env::var("EPISCO_BIBLE") else { return };
    let bible = BibleText::from_json(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut failures = Vec::new();
    let mut checked = std::collections::BTreeSet::new();
    let mut d = Date::new(2025, 11, 30);
    while d < Date::new(2028, 12, 3) {
        let p = lectionary::for_date(d, None);
        for o in &p.observances {
            for item in lectionary::for_date(d, Some(o)).items {
                if matches!(item.kind, ProperKind::Collect | ProperKind::Psalm)
                    || !checked.insert(item.citation.clone())
                {
                    continue;
                }
                match bible.passage(&item.citation, true) {
                    Ok(t) if t.split_whitespace().count() >= 8 => {}
                    Ok(t) => failures.push(format!("{}: short: {t}", item.citation)),
                    Err(e) => failures.push(format!("{}: {e}", item.citation)),
                }
            }
        }
        d = d.add_days(1);
    }
    println!("checked {} citations", checked.len());
    if let Ok(list) = std::env::var("EPISCO_BIBLE_SHOW") {
        for c in list.split('|') {
            println!("== {c}\n{}\n", bible.passage(c, true).unwrap());
        }
    }
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
}

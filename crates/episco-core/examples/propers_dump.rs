//! Shows what Import propers finds in a page copied from the lectionary site:
//!
//!     cargo run -p episco-core --example propers_dump -- page.txt

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: propers_dump <page.txt>");
        std::process::exit(2);
    };
    let text = std::fs::read_to_string(&path).expect("could not read the file");
    let p = episco_core::propers::parse(&text);
    println!("Day: {}", p.day.as_deref().unwrap_or("(not found)"));
    for i in p.items {
        let what = if i.citation.is_empty() { i.text.chars().take(60).collect::<String>() + "…" } else { i.citation };
        let variant = i.variant.map(|v| format!(" [{v}]")).unwrap_or_default();
        let alt = if i.alternative { " (or)" } else { "" };
        let verses = if i.verses.is_empty() { String::new() } else { format!(" — {} verses", i.verses.len()) };
        println!("{:?}{variant}{alt}: {what}{verses}", i.kind);
    }
}

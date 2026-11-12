//! Paths that work on every computer sharing a parish library folder.
//!
//! The folder is synced to a different place on each computer (e.g.
//! `/Users/rector/Google Drive/Parish Library` and `G:\My Drive\Parish
//! Library`), so a picture inside it is stored as `library:media/x.png`, relative
//! to the folder, rather than as a full path. [`Service::make_portable`] does
//! this before a service is saved; [`resolve`] turns it back into a path.

use std::path::{Path, PathBuf};

use crate::document::Block;
use crate::look::Look;
use crate::service::{CueContent, Service};

/// Marks a path relative to the parish library folder.
pub const PREFIX: &str = "library:";

fn slashes(p: &str) -> String {
    p.replace('\\', "/")
}

/// `value` as `library:…` if it is a full path inside `root`.
pub fn to_portable(value: &str, root: &Path) -> Option<String> {
    if value.starts_with(PREFIX) {
        return None;
    }
    let v = slashes(value);
    let r = slashes(&root.to_string_lossy());
    let r = r.trim_end_matches('/');
    if r.is_empty() {
        return None;
    }
    // Windows paths compare without regard to case.
    let inside =
        |v: &str| v.len() > r.len() + 1 && v[..r.len()].eq_ignore_ascii_case(r) && v.as_bytes()[r.len()] == b'/';
    inside(&v).then(|| format!("{PREFIX}{}", &v[r.len() + 1..]))
}

/// The full path for a `library:…` value (others are returned as they are).
pub fn resolve(value: &str, root: Option<&Path>) -> Option<PathBuf> {
    match value.strip_prefix(PREFIX) {
        Some(rel) => Some(root?.join(rel)),
        None => Some(PathBuf::from(value)),
    }
}

fn fix(value: &mut String, root: &Path) -> bool {
    match to_portable(value, root) {
        Some(p) => {
            *value = p;
            true
        }
        None => false,
    }
}

fn fix_look(look: &mut Look, root: &Path) -> bool {
    look.background.as_mut().is_some_and(|b| fix(b, root))
}

impl Service {
    /// Rewrites full paths inside the library folder `root` (pictures in
    /// items, media items, backgrounds) as `library:…`. Items whose text
    /// changed are marked modified, so saving writes them. Returns whether
    /// anything changed.
    pub fn make_portable(&mut self, root: &Path) -> bool {
        let mut any = false;
        for looks in self.looks.values_mut() {
            any |= fix_look(&mut looks.sanctuary, root);
            any |= fix_look(&mut looks.stream, root);
        }
        for cue in &mut self.cues {
            let mut changed = fix_look(&mut cue.overrides.looks.sanctuary, root);
            changed |= fix_look(&mut cue.overrides.looks.stream, root);
            if let CueContent::Liturgy { document, .. } = &mut cue.content {
                let mut doc_changed = false;
                for b in &mut document.blocks {
                    if let Block::Image { src, .. } = b {
                        doc_changed |= fix(src, root);
                    }
                }
                for (k, v) in &mut document.meta.extra {
                    if k == "media" {
                        doc_changed |= fix(v, root);
                    }
                }
                if doc_changed {
                    cue.modified = true;
                }
                changed |= doc_changed;
            }
            any |= changed;
        }
        if any {
            self.modified = true;
            self.rebuild_all();
        }
        any
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::{Cue, CueOverrides};
    use crate::LiturgyDocument;

    #[test]
    fn paths_inside_the_library_become_portable() {
        let root = Path::new("/Users/rector/Google Drive/Parish Library");
        assert_eq!(
            to_portable("/Users/rector/Google Drive/Parish Library/media/Psalm 23.png", root).as_deref(),
            Some("library:media/Psalm 23.png")
        );
        assert_eq!(to_portable("/Users/rector/Pictures/x.png", root), None);
        assert_eq!(to_portable("/Users/rector/Google Drive/Parish Library2/x.png", root), None);
        assert_eq!(
            to_portable(r"G:\My Drive\Parish Library\media\a.png", Path::new(r"g:\My Drive\Parish Library")).as_deref(),
            Some("library:media/a.png")
        );
        assert_eq!(resolve("library:media/a.png", Some(Path::new("/lib"))), Some(PathBuf::from("/lib/media/a.png")));
        assert_eq!(resolve("library:x", None), None);
        assert_eq!(resolve("/a/b.png", None), Some(PathBuf::from("/a/b.png")));

        let doc: LiturgyDocument =
            "---\ntitle: Psalm\nmedia: /L/media/v.mp4\n---\n![Antiphon](/L/media/a.png)\n\n[1] Text\n".parse().unwrap();
        let mut s = Service::default();
        let mut o = CueOverrides::default();
        o.looks.sanctuary.background = Some("/L/backgrounds/b.jpg".into());
        s.insert_cue(0, Cue::liturgy("", doc, None, o));
        s.cues[0].modified = false;
        assert!(s.make_portable(Path::new("/L")));
        let c = &s.cues[0];
        assert!(c.modified);
        assert_eq!(c.document().unwrap().meta.extra("media"), Some("library:media/v.mp4"));
        assert!(matches!(&c.document().unwrap().blocks[0], Block::Image { src, .. } if src == "library:media/a.png"));
        assert_eq!(c.overrides.looks.sanctuary.background.as_deref(), Some("library:backgrounds/b.jpg"));
        assert_eq!(c.sanctuary.background, "library:backgrounds/b.jpg");
        assert!(!s.make_portable(Path::new("/L")), "nothing left to change");
    }
}

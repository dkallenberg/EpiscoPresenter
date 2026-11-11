//! Live presentation state and the commands that change it.
//!
//! The master node owns the authoritative [`PresentationState`]. Every
//! control surface (operator UI, slave node, tablet remote) sends
//! [`Command`]s; the master applies them with [`PresentationState::apply`]
//! and broadcasts the result. Keeping this a pure function of
//! `(state, command, outline)` makes all nodes agree by construction.

use serde::{Deserialize, Serialize};

/// The currently projected slide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CuePosition {
    pub cue: usize,
    pub slide: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PresentationState {
    /// Monotonic counter, bumped on every change. Lets clients discard stale
    /// or duplicate updates.
    pub revision: u64,
    /// `None` before the first cue is taken (the pre-service state, during
    /// which the announcement loop runs).
    pub active: Option<CuePosition>,
    /// Both outputs fully black.
    pub blackout: bool,
    /// Text layer hidden; backgrounds remain. Reset by navigation.
    pub text_cleared: bool,
    /// Media / background layer hidden. Reset by navigation.
    pub media_cleared: bool,
    /// Message shown to the people on stage (operator → clergy).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_message: Option<String>,
}

/// An operator action. Serialized as `{"type": "next"}`,
/// `{"type": "go_to", "cue": 3, "slide": 0}`, etc.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    Next,
    Prev,
    GoTo {
        cue: usize,
        slide: usize,
    },
    /// Return to the pre-service state.
    Reset,
    ToggleBlackout,
    SetBlackout {
        on: bool,
    },
    ClearText,
    ClearMedia,
    /// Hide text and media and lift blackout: an empty background.
    ClearAll,
    StageMessage {
        text: String,
    },
    ClearStageMessage,
}

impl PresentationState {
    /// Applies `cmd` given the slide count of each cue. Returns `true` (and
    /// bumps `revision`) if anything changed.
    pub fn apply(&mut self, cmd: &Command, outline: &[usize]) -> bool {
        let before = self.clone();
        match cmd {
            Command::Next => {
                if let Some(pos) = next_position(self.active, outline) {
                    self.navigate(pos);
                }
            }
            Command::Prev => {
                if let Some(pos) = prev_position(self.active, outline) {
                    self.navigate(pos);
                }
            }
            Command::GoTo { cue, slide } => {
                if outline.get(*cue).is_some_and(|&n| *slide < n.max(1)) {
                    self.navigate(CuePosition { cue: *cue, slide: *slide });
                }
            }
            Command::Reset => {
                self.active = None;
                self.text_cleared = false;
                self.media_cleared = false;
            }
            Command::ToggleBlackout => self.blackout = !self.blackout,
            Command::SetBlackout { on } => self.blackout = *on,
            Command::ClearText => self.text_cleared = true,
            Command::ClearMedia => self.media_cleared = true,
            Command::ClearAll => {
                self.text_cleared = true;
                self.media_cleared = true;
                self.blackout = false;
            }
            Command::StageMessage { text } => {
                let t = text.trim();
                self.stage_message = (!t.is_empty()).then(|| t.to_string());
            }
            Command::ClearStageMessage => self.stage_message = None,
        }
        let changed = *self != before;
        if changed {
            self.revision += 1;
        }
        changed
    }

    /// Re-validates the active position after the service is replaced.
    /// Returns `true` (and bumps `revision`) if the state changed.
    pub fn reconcile(&mut self, outline: &[usize]) -> bool {
        let valid = self.active.map(|p| outline.get(p.cue).is_some_and(|&n| p.slide < n.max(1))).unwrap_or(true);
        if valid {
            return false;
        }
        self.active = None;
        self.revision += 1;
        true
    }

    fn navigate(&mut self, pos: CuePosition) {
        self.active = Some(pos);
        self.text_cleared = false;
        self.media_cleared = false;
    }
}

fn next_position(active: Option<CuePosition>, outline: &[usize]) -> Option<CuePosition> {
    match active {
        None => (!outline.is_empty()).then_some(CuePosition { cue: 0, slide: 0 }),
        Some(CuePosition { cue, slide }) => {
            let count = outline.get(cue).copied().unwrap_or(0).max(1);
            if slide + 1 < count {
                Some(CuePosition { cue, slide: slide + 1 })
            } else if cue + 1 < outline.len() {
                Some(CuePosition { cue: cue + 1, slide: 0 })
            } else {
                None
            }
        }
    }
}

fn prev_position(active: Option<CuePosition>, outline: &[usize]) -> Option<CuePosition> {
    let CuePosition { cue, slide } = active?;
    if slide > 0 {
        Some(CuePosition { cue, slide: slide - 1 })
    } else if cue > 0 {
        let prev = cue - 1;
        let last = outline.get(prev).copied().unwrap_or(1).max(1) - 1;
        Some(CuePosition { cue: prev, slide: last })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(cue: usize, slide: usize) -> Option<CuePosition> {
        Some(CuePosition { cue, slide })
    }

    #[test]
    fn next_walks_slides_then_cues_and_stops_at_end() {
        let outline = [2, 0, 1];
        let mut s = PresentationState::default();
        let mut seen = vec![];
        while s.apply(&Command::Next, &outline) {
            seen.push(s.active);
        }
        // An empty cue still occupies one position.
        assert_eq!(seen, vec![pos(0, 0), pos(0, 1), pos(1, 0), pos(2, 0)]);
        assert_eq!(s.revision, 4);
    }

    #[test]
    fn prev_lands_on_last_slide_of_previous_cue() {
        let outline = [3, 1];
        let mut s = PresentationState { active: pos(1, 0), ..Default::default() };
        assert!(s.apply(&Command::Prev, &outline));
        assert_eq!(s.active, pos(0, 2));
        s.active = pos(0, 0);
        assert!(!s.apply(&Command::Prev, &outline));
    }

    #[test]
    fn navigation_restores_cleared_layers_but_not_blackout() {
        let outline = [2];
        let mut s = PresentationState::default();
        s.apply(&Command::Next, &outline);
        s.apply(&Command::ClearText, &outline);
        s.apply(&Command::ToggleBlackout, &outline);
        assert!(s.text_cleared && s.blackout);
        s.apply(&Command::Next, &outline);
        assert!(!s.text_cleared);
        assert!(s.blackout);
    }

    #[test]
    fn invalid_goto_is_ignored_and_does_not_bump_revision() {
        let outline = [2];
        let mut s = PresentationState::default();
        assert!(!s.apply(&Command::GoTo { cue: 5, slide: 0 }, &outline));
        assert!(!s.apply(&Command::GoTo { cue: 0, slide: 2 }, &outline));
        assert!(s.apply(&Command::GoTo { cue: 0, slide: 1 }, &outline));
        assert_eq!(s.revision, 1);
    }

    #[test]
    fn stage_messages() {
        let mut s = PresentationState::default();
        assert!(!s.apply(&Command::StageMessage { text: "  ".into() }, &[]));
        assert!(s.apply(&Command::StageMessage { text: " Slow down ".into() }, &[]));
        assert_eq!(s.stage_message.as_deref(), Some("Slow down"));
        assert!(s.apply(&Command::ClearStageMessage, &[]));
    }

    #[test]
    fn reconcile_drops_out_of_range_position() {
        let mut s = PresentationState { active: pos(3, 0), ..Default::default() };
        assert!(s.reconcile(&[1, 1]));
        assert_eq!(s.active, None);
        assert!(!s.reconcile(&[1, 1]));
    }

    #[test]
    fn commands_have_stable_json() {
        assert_eq!(serde_json::to_string(&Command::Next).unwrap(), r#"{"type":"next"}"#);
        let c: Command = serde_json::from_str(r#"{"type":"go_to","cue":2,"slide":1}"#).unwrap();
        assert_eq!(c, Command::GoTo { cue: 2, slide: 1 });
    }
}

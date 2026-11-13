//! Speaker roles used in liturgical dialogues and their presentation styles.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A speaker role attached to a block of liturgical text.
///
/// Serialized as a plain string (`"Celebrant"`, `"People"`, ...). Roles that
/// are not part of the standard set are preserved verbatim as [`Role::Custom`]
/// so that files using e.g. `[Cantor]` round-trip without loss.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "String", from = "String")]
pub enum Role {
    Celebrant,
    Priest,
    Bishop,
    Deacon,
    Reader,
    /// The congregation. Always rendered bold / full black.
    People,
    /// Everyone together, priest and people (the Creed, the Lord's Prayer).
    /// Bold like the people's parts.
    All,
    /// Instructions to the participants; not spoken aloud.
    Rubric,
    /// Any non-standard role name, preserved as written.
    Custom(String),
}

impl Role {
    /// The standard roles offered by the editor's role selector, in menu order.
    pub const STANDARD: [Role; 8] = [
        Role::Celebrant,
        Role::Priest,
        Role::Bishop,
        Role::Deacon,
        Role::Reader,
        Role::People,
        Role::All,
        Role::Rubric,
    ];

    /// Parses a role name case-insensitively, accepting common aliases.
    ///
    /// Unknown names become [`Role::Custom`] with surrounding whitespace trimmed.
    pub fn parse(name: &str) -> Role {
        let trimmed = name.trim();
        match trimmed.to_ascii_lowercase().as_str() {
            "celebrant" | "officiant" => Role::Celebrant,
            "priest" | "presider" => Role::Priest,
            "bishop" => Role::Bishop,
            "deacon" => Role::Deacon,
            "reader" | "lector" => Role::Reader,
            "people" | "congregation" => Role::People,
            "all" | "together" => Role::All,
            "rubric" => Role::Rubric,
            _ => Role::Custom(trimmed.to_string()),
        }
    }

    /// Display label, e.g. `"Celebrant"`.
    pub fn label(&self) -> &str {
        match self {
            Role::Celebrant => "Celebrant",
            Role::Priest => "Priest",
            Role::Bishop => "Bishop",
            Role::Deacon => "Deacon",
            Role::Reader => "Reader",
            Role::People => "People",
            Role::All => "All",
            Role::Rubric => "Rubric",
            Role::Custom(name) => name,
        }
    }

    /// Who says this part at a service: with a bishop presiding the
    /// Celebrant's and Priest's parts are the Bishop's; with no deacon the
    /// Deacon's parts are said by a priest ("the Deacon or a Priest").
    pub fn at_service(&self, bishop: bool, no_deacon: bool) -> Role {
        match self {
            Role::Celebrant | Role::Priest if bishop => Role::Bishop,
            Role::Deacon if no_deacon => Role::Priest,
            r => r.clone(),
        }
    }

    /// True for text spoken by the congregation (included in bulletins, bold on screen).
    pub fn is_congregational(&self) -> bool {
        matches!(self, Role::People | Role::All)
    }

    /// True for roles whose text is not spoken (rubrics).
    pub fn is_rubric(&self) -> bool {
        matches!(self, Role::Rubric)
    }

    /// Default formatting for this role across editor and display outputs.
    pub fn style(&self) -> RoleStyle {
        match self {
            Role::People | Role::All => RoleStyle {
                text_color: "#000000",
                bold: true,
                italic: false,
                editor_bar_color: "#fff4c2",
                editor_accent_color: "#c99a06",
            },
            Role::Rubric => RoleStyle {
                text_color: "#8b1a1a",
                bold: false,
                italic: true,
                editor_bar_color: "#f3e3e3",
                editor_accent_color: "#a33a3a",
            },
            Role::Celebrant | Role::Priest | Role::Bishop => RoleStyle {
                text_color: "#404040",
                bold: false,
                italic: false,
                editor_bar_color: "#e3ecf7",
                editor_accent_color: "#2f5f9e",
            },
            Role::Deacon => RoleStyle {
                text_color: "#404040",
                bold: false,
                italic: false,
                editor_bar_color: "#e5f3e5",
                editor_accent_color: "#2e7d32",
            },
            Role::Reader => RoleStyle {
                text_color: "#404040",
                bold: false,
                italic: false,
                editor_bar_color: "#efe6f7",
                editor_accent_color: "#6a3d9a",
            },
            Role::Custom(_) => RoleStyle {
                text_color: "#404040",
                bold: false,
                italic: false,
                editor_bar_color: "#ececec",
                editor_accent_color: "#6b6b6b",
            },
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl From<Role> for String {
    fn from(role: Role) -> String {
        role.label().to_string()
    }
}

impl From<String> for Role {
    fn from(s: String) -> Role {
        Role::parse(&s)
    }
}

/// Presentation defaults for a role.
///
/// Sizes are fixed by the display spec (role labels 48pt italic, body text 55pt);
/// only colour, weight and slant vary per role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RoleStyle {
    /// Body text colour on the sanctuary display.
    pub text_color: &'static str,
    pub bold: bool,
    pub italic: bool,
    /// Background colour of the role's row in the operator's block editor.
    pub editor_bar_color: &'static str,
    /// Strong edge / label colour for the role's editor row.
    pub editor_accent_color: &'static str,
}

/// Point size of role labels on the sanctuary display (rendered italic).
pub const ROLE_LABEL_PT: u32 = 48;
/// Point size of body text on the sanctuary display.
pub const BODY_TEXT_PT: u32 = 55;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_case_insensitively_with_aliases() {
        assert_eq!(Role::parse("PRIEST"), Role::Priest);
        assert_eq!(Role::parse(" people "), Role::People);
        assert_eq!(Role::parse("All"), Role::All);
        assert_eq!(Role::parse("congregation"), Role::People);
        assert_eq!(Role::parse("Lector"), Role::Reader);
        assert_eq!(Role::parse("Cantor"), Role::Custom("Cantor".into()));
    }

    #[test]
    fn serializes_as_plain_string() {
        let json = serde_json::to_string(&vec![Role::People, Role::Custom("Cantor".into())]).unwrap();
        assert_eq!(json, r#"["People","Cantor"]"#);
        let back: Vec<Role> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, vec![Role::People, Role::Custom("Cantor".into())]);
    }

    #[test]
    fn people_are_bold_black() {
        let s = Role::People.style();
        assert!(s.bold);
        assert_eq!(s.text_color, "#000000");
        assert_eq!(Role::Priest.style().text_color, "#404040");
    }
}

//! A page: one markdown document that declares one Diataxis type.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{Error, Result};

/// The four Diataxis types, in the order a reader meets them.
///
/// Tutorial and how-to guide inform action; reference and explanation
/// inform cognition. Tutorial and explanation serve a reader acquiring
/// skill; how-to guide and reference serve one applying it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    Tutorial,
    HowTo,
    Reference,
    Explanation,
}

impl Kind {
    /// Every type, in Diataxis order. `map` prints them this way.
    pub const ALL: [Kind; 4] = [
        Kind::Tutorial,
        Kind::HowTo,
        Kind::Reference,
        Kind::Explanation,
    ];

    /// The canonical name, and what a new page is written with.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Tutorial => "tutorial",
            Kind::HowTo => "how-to",
            Kind::Reference => "reference",
            Kind::Explanation => "explanation",
        }
    }

    /// What the type informs: action or cognition.
    #[must_use]
    pub fn informs(self) -> &'static str {
        match self {
            Kind::Tutorial | Kind::HowTo => "action",
            Kind::Reference | Kind::Explanation => "cognition",
        }
    }

    /// What the type serves: acquisition of skill, or application.
    #[must_use]
    pub fn serves(self) -> &'static str {
        match self {
            Kind::Tutorial | Kind::Explanation => "acquisition",
            Kind::HowTo | Kind::Reference => "application",
        }
    }

    /// The one-line brief the compass prints.
    #[must_use]
    pub fn brief(self) -> &'static str {
        match self {
            Kind::Tutorial => {
                "a lesson: the reader learns by doing, and the writer owns the outcome"
            }
            Kind::HowTo => "directions: the reader is competent and already knows the goal",
            Kind::Reference => "neutral description of the machinery, consulted during work",
            Kind::Explanation => "a discursive treatment of one topic, read away from the work",
        }
    }
}

impl std::str::FromStr for Kind {
    type Err = Error;

    /// Accepts what the four sibling repositories already write.
    ///
    /// `guide` is the spelling those pages carry, and a tool that
    /// refused it would refuse every page it was built to read.
    fn from_str(s: &str) -> Result<Kind> {
        match s.trim().to_ascii_lowercase().as_str() {
            "tutorial" => Ok(Kind::Tutorial),
            "how-to" | "howto" | "how_to" | "how-to-guide" | "guide" => Ok(Kind::HowTo),
            "reference" => Ok(Kind::Reference),
            "explanation" | "discussion" | "background" => Ok(Kind::Explanation),
            other => Err(Error::UnknownKind(other.to_string())),
        }
    }
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for Kind {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Kind {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Kind, D::Error> {
        let raw = String::deserialize(d)?;
        raw.parse().map_err(serde::de::Error::custom)
    }
}

/// What a page declares about itself.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PageFrontmatter {
    pub title: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,

    /// The key is `type`, because thirteen existing pages already write
    /// it that way.
    #[serde(rename = "type")]
    pub kind: Kind,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// A page as it sits in a store.
#[derive(Debug, Clone)]
pub struct Page {
    /// The file stem. A page id is its published path, so it carries no
    /// random prefix: a reader bookmarks `getting-started.html`.
    pub id: String,
    pub frontmatter: PageFrontmatter,
    pub body: String,
    /// The form the file is written in, so an edit writes it back the
    /// same way.
    pub fence: mdstore::Fence,
}

impl Page {
    #[must_use]
    pub fn kind(&self) -> Kind {
        self.frontmatter.kind
    }

    #[must_use]
    pub fn title(&self) -> &str {
        &self.frontmatter.title
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guide_parses_as_a_how_to() {
        assert_eq!("guide".parse::<Kind>().unwrap(), Kind::HowTo);
        assert_eq!("How-To".parse::<Kind>().unwrap(), Kind::HowTo);
    }

    #[test]
    fn an_unknown_type_is_refused() {
        assert!("overview".parse::<Kind>().is_err());
    }

    #[test]
    fn a_kind_round_trips_through_its_canonical_name() {
        for kind in Kind::ALL {
            assert_eq!(kind.as_str().parse::<Kind>().unwrap(), kind);
        }
    }

    #[test]
    fn the_two_questions_separate_the_four_types() {
        let mut seen = std::collections::HashSet::new();
        for kind in Kind::ALL {
            assert!(seen.insert((kind.informs(), kind.serves())));
        }
        assert_eq!(seen.len(), 4);
    }
}

use serde::{Deserialize, Serialize};

/// `diataxis.yml`, at the root of a documentation set.
#[derive(Debug, Deserialize, Serialize)]
pub struct DiataxisConfig {
    /// Where the pages live, relative to the root.
    ///
    /// The default is `docs`, not a hidden directory. A page is the
    /// published artefact, unlike an issue or a note, and every reader
    /// and every site build looks for it there.
    pub docs_dir: String,

    /// The title a rendered set carries.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
}

impl Default for DiataxisConfig {
    fn default() -> Self {
        Self {
            docs_dir: "docs".into(),
            title: String::new(),
        }
    }
}

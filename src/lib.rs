//! Diataxis documentation sets as typed markdown.
//!
//! Three halves, each behind a feature, because three kinds of consumer
//! want different amounts of this crate.
//!
//! `store`: read and write the pages of one documentation set, and
//! serve a set compiled into a binary. [`Repo`], [`Page`], [`check`],
//! [`docs::DocSet`].
//!
//! `cli`: the binary, and the clap surface it is built from. A tool
//! that parses its own arguments turns this off and calls
//! [`docs::DocSet::render`] with a [`docs::Request`].
//!
//! `build`: the build-script helper, which carries no dependencies.
//! See [`build`].

#[cfg(feature = "build")]
pub mod build;

#[cfg(feature = "store")]
pub mod check;
#[cfg(feature = "store")]
pub mod config;
#[cfg(feature = "store")]
pub mod docs;
#[cfg(feature = "store")]
pub mod error;
#[cfg(feature = "store")]
pub mod page;
#[cfg(feature = "store")]
pub mod repo;
#[cfg(feature = "store")]
pub mod selector;

#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "cli")]
pub mod mangen;

#[cfg(feature = "store")]
pub use docs::{DocPage, DocSet, Request};
#[cfg(feature = "store")]
pub use error::{Error, Result};
#[cfg(feature = "store")]
pub use page::{Kind, Page, PageFrontmatter};
#[cfg(feature = "store")]
pub use repo::{CreatePageOptions, EditPageOptions, Repo, SearchResult};

/// Every page of this project's documentation set, compiled in.
///
/// One line, and no list. The build script wrote the array; this reads
/// it. A page added to the pages directory is in the next build.
///
/// ```ignore
/// // build.rs
/// fn main() { diataxis::build::embed_docs().unwrap(); }
///
/// // anywhere in the crate
/// static DOCS: &[(&str, &str)] = diataxis::embedded_docs!();
/// ```
#[macro_export]
macro_rules! embedded_docs {
    () => {
        include!(concat!(env!("OUT_DIR"), "/diataxis_docs.rs"))
    };
}

/// Compile named pages into the binary, as `(path, content)` pairs for
/// [`DocSet::from_embedded`](crate::docs::DocSet::from_embedded).
///
/// [`embedded_docs!`] is the usual call, and it needs no list. Name the
/// pages here only when the order matters, or when a subset of the
/// directory belongs in this binary.
///
/// A path is relative to the file that calls the macro, the way
/// `include_str!` is.
///
/// ```ignore
/// static DOCS: &[(&str, &str)] = diataxis::embed_docs![
///     "../docs/what-is-acme.md",
///     "../docs/getting-started.md",
/// ];
/// ```
#[macro_export]
macro_rules! embed_docs {
    ($($path:literal),* $(,)?) => {
        &[$(($path, include_str!($path))),*]
    };
}

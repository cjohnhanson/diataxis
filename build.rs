//! This crate embeds its own documentation the way a consumer does.
//!
//! A consumer takes `diataxis` as a build-dependency with the `build`
//! feature. This crate cannot depend on itself, so it points a module
//! at the same source file.

#[path = "src/build.rs"]
mod embed;

fn main() {
    embed::embed_docs().expect("embed the documentation set");
}

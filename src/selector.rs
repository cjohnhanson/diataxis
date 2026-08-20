//! Naming one page from a partial id.

use crate::error::{Error, Result};
use crate::page::Page;

/// The page a selector names.
///
/// An exact id wins outright. Otherwise a unique prefix names a page,
/// and an ambiguous one is an error that lists what it matched. A
/// selector that quietly picked the first match would edit the wrong
/// page.
pub fn pick(pages: Vec<Page>, selector: &str) -> Result<Page> {
    let mut matches: Vec<Page> = Vec::new();
    for page in pages {
        if page.id == selector {
            return Ok(page);
        }
        if page.id.starts_with(selector) {
            matches.push(page);
        }
    }
    match matches.len() {
        0 => Err(Error::NoSuchPage(selector.to_string())),
        1 => Ok(matches.remove(0)),
        _ => {
            let names: Vec<&str> = matches.iter().map(|p| p.id.as_str()).collect();
            Err(Error::AmbiguousPage(selector.to_string(), names.join(", ")))
        }
    }
}

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
    // An empty selector is a zero-length prefix, so it matches every
    // page. In a one-page set that is a unique match, and
    // `page delete "$ID"` with ID unset deleted the only page at exit 0.
    // A two-page set refused the same call, which hid it.
    if selector.is_empty() {
        return Err(Error::NoSuchPage("an empty name".to_string()));
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::{Kind, PageFrontmatter};

    fn page(id: &str) -> Page {
        Page {
            id: id.to_string(),
            frontmatter: PageFrontmatter {
                title: id.to_string(),
                description: String::new(),
                kind: Kind::Explanation,
                tags: Vec::new(),
            },
            body: String::new(),
            fence: mdstore::Fence::Yaml,
        }
    }

    #[test]
    fn an_exact_id_wins_over_a_longer_page_it_prefixes() {
        // `install` must reach `install`, not the two pages it prefixes.
        let pages = vec![page("install"), page("install-nix"), page("install-brew")];
        assert_eq!(pick(pages, "install").expect("exact").id, "install");
    }

    #[test]
    fn an_ambiguous_prefix_refuses_and_names_every_match() {
        // Taking the first match silently edits or deletes a page the
        // reader did not name. The refusal has to carry the candidates,
        // because the reader's next move is to pick one.
        let pages = vec![page("install-nix"), page("install-brew")];
        let err = pick(pages, "install").expect_err("ambiguous");
        let text = err.to_string();
        assert!(text.contains("install-nix"), "{text}");
        assert!(text.contains("install-brew"), "{text}");
    }

    #[test]
    fn a_prefix_matching_one_page_resolves() {
        let pages = vec![page("install-nix"), page("release")];
        assert_eq!(pick(pages, "install").expect("one").id, "install-nix");
    }

    #[test]
    fn an_empty_selector_refuses_even_in_a_one_page_set() {
        // The dangerous case: one page, so an empty string is a unique
        // prefix match. `page delete "$ID"` with ID unset deleted it.
        let err = pick(vec![page("only")], "").expect_err("empty refused");
        assert!(matches!(err, Error::NoSuchPage(_)), "{err:?}");
    }

    #[test]
    fn an_empty_selector_refuses_in_a_larger_set() {
        let err = pick(vec![page("one"), page("two")], "").expect_err("empty refused");
        assert!(matches!(err, Error::NoSuchPage(_)), "{err:?}");
    }

    #[test]
    fn a_selector_matching_nothing_refuses() {
        let err = pick(vec![page("install")], "absent").expect_err("none");
        assert!(matches!(err, Error::NoSuchPage(_)), "{err:?}");
    }
}

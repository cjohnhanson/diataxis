//! What a documentation set can be checked for mechanically.
//!
//! Two kinds of finding. A fault is provable from the files: a page
//! that does not parse, a link that resolves to nothing, a how-to title
//! that does not state a goal. Advice is a gap worth seeing that is not
//! wrong on its own, such as a type with no pages.
//!
//! Nothing here judges whether prose in a reference page is really
//! explanation. That needs a reader, and a check that guessed would
//! report confident noise.

use camino::Utf8PathBuf;

use crate::error::Result;
use crate::page::{Kind, Page, PageFrontmatter};
use crate::repo::Repo;

/// How much a finding matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Provable from the files.
    Fault,
    /// A gap worth seeing, not an error.
    Advice,
}

/// One thing the check found.
#[derive(Debug)]
pub struct Finding {
    pub severity: Severity,
    /// The page it concerns, or the set as a whole.
    pub subject: String,
    pub message: String,
}

/// Every finding, faults first.
pub fn check(root: &camino::Utf8Path) -> Result<Vec<Finding>> {
    let repo = Repo::open(root)?;
    let mut findings = Vec::new();
    let pages = repo.list()?;

    findings.extend(unreadable_pages(&repo)?);
    findings.extend(how_to_titles(&pages));
    findings.extend(broken_links(&repo, &pages)?);
    findings.extend(unavailable_sets(root)?);
    findings.extend(missing_types(&pages));

    findings.sort_by_key(|f| match f.severity {
        Severity::Fault => 0,
        Severity::Advice => 1,
    });
    Ok(findings)
}

/// A declared set that could not be loaded.
///
/// A check over one set passes over a qualified reference, and cannot
/// say whether it resolves. An unavailable set is why: nothing in it is
/// readable, so every reference into it is unanswerable.
///
/// This once claimed to report a cross-set reference that names no
/// page. It never did. It read the unavailable-set labels and filtered
/// them for a `:`, which those labels do not carry, so it returned
/// nothing on every input and hid the unavailable set as well. mdstore
/// exposes no per-reference resolution across a closure, so the honest
/// answer is the one this gives.
fn unavailable_sets(root: &camino::Utf8Path) -> Result<Vec<Finding>> {
    let workspace = crate::workspace::Workspace::open(root)?;
    Ok(workspace
        .unavailable_sets()
        .into_iter()
        .map(|label| Finding {
            severity: Severity::Fault,
            subject: "the closure".to_string(),
            message: format!(
                "the set {label} did not load, so no reference into it can be checked"
            ),
        })
        .collect())
}

/// A markdown file in the pages directory that is not a page.
///
/// `list` skips these, so without this check an untyped file is
/// invisible: it holds no type, and every command that reads types
/// passes over it in silence.
fn unreadable_pages(repo: &Repo) -> Result<Vec<Finding>> {
    let dir = repo.docs_dir()?;
    let mut findings = Vec::new();
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(findings),
        Err(e) => return Err(e.into()),
    };
    let mut paths: Vec<Utf8PathBuf> = Vec::new();
    for entry in entries {
        if let Ok(path) = Utf8PathBuf::from_path_buf(entry?.path())
            && path.extension() == Some("md")
        {
            paths.push(path);
        }
    }
    paths.sort();
    for path in paths {
        let text = std::fs::read_to_string(&path)?;
        if let Err(e) = mdstore::parse_any::<PageFrontmatter>(&text) {
            findings.push(Finding {
                severity: Severity::Fault,
                subject: path.file_name().unwrap_or_default().to_string(),
                message: format!("not a page: {e}"),
            });
        }
    }
    Ok(findings)
}

/// A how-to title states the goal.
///
/// The framework's own rule: "How to integrate application performance
/// monitoring", not "Integrating application performance monitoring".
/// A reader scanning a list has to see which page does the thing they
/// want.
fn how_to_titles(pages: &[Page]) -> Vec<Finding> {
    pages
        .iter()
        .filter(|p| p.kind() == Kind::HowTo)
        .filter(|p| !p.title().to_ascii_lowercase().starts_with("how to"))
        .map(|p| Finding {
            severity: Severity::Fault,
            subject: p.id.clone(),
            message: format!(
                "a how-to title states a goal: \"{}\" does not start with \"How to\"",
                p.title()
            ),
        })
        .collect()
}

/// Every type carries at least one page.
fn missing_types(pages: &[Page]) -> Vec<Finding> {
    Kind::ALL
        .iter()
        .filter(|kind| !pages.iter().any(|p| p.kind() == **kind))
        .map(|kind| Finding {
            severity: Severity::Advice,
            subject: "the set".to_string(),
            message: format!("no {kind} pages"),
        })
        .collect()
}

/// A link that resolves to nothing.
///
/// Two forms are checked: a `[[id]]` reference, and a markdown link to
/// a `.md` file beside the page. An external URL is left alone.
fn broken_links(repo: &Repo, pages: &[Page]) -> Result<Vec<Finding>> {
    let dir = repo.docs_dir()?;
    let known: Vec<&str> = pages.iter().map(|p| p.id.as_str()).collect();
    let mut findings = Vec::new();
    for page in pages {
        let (wiki, links) = prose_targets(&page.body);
        for target in wiki {
            // A qualified reference names a declared store, and this
            // check reads one store, so it cannot resolve it.
            if target.contains(':') {
                continue;
            }
            if !known.contains(&target.as_str()) {
                findings.push(Finding {
                    severity: Severity::Fault,
                    subject: page.id.clone(),
                    message: format!("[[{target}]] names no page"),
                });
            }
        }
        for target in links {
            if !dir.join(&target).exists() {
                findings.push(Finding {
                    severity: Severity::Fault,
                    subject: page.id.clone(),
                    message: format!("{target} names no file"),
                });
            }
        }
    }
    Ok(findings)
}

/// Every reference and every relative `.md` link a page's prose names.
///
/// A link check reads prose only. Documentation quotes syntax
/// constantly, and `[[note-id]]` inside backticks is an example of a
/// reference rather than a reference.
///
/// pulldown-cmark decides what is prose. Three hand-rolled scanners
/// stood here before: one stripped fenced blocks by counting backtick
/// runs at the start of a line, one stripped inline spans, and one
/// found link targets by searching for `](`. Together they reported a
/// reference inside an indented code block and inside a fenced block
/// within a blockquote, and they missed `[a](page.md "a title")`
/// because the raw span did not end in `.md`. A CommonMark parser
/// answers all three, and this crate is not the place to write one.
fn prose_targets(body: &str) -> (Vec<String>, Vec<String>) {
    let (refs, links) = scan_prose(body);
    (refs.into_iter().map(|r| r.target).collect(), links)
}

/// One `[[reference]]` in a page's prose, with the bytes it spans in
/// the body, so a caller can replace it and leave every other byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProseRef {
    /// From the first `[` to the last `]`.
    pub range: std::ops::Range<usize>,
    /// The page it names: `id`, or `alias:id` for a page in another set.
    pub target: String,
    /// The text after a `|`, as written: `[[id|the text]]`.
    pub text: Option<String>,
    /// Written as an embed, `![[id]]`, rather than a link.
    pub embed: bool,
}

/// Every `[[reference]]` in a page's prose, in body order.
///
/// pulldown-cmark's wikilink extension reads them, the same grammar
/// for `check`, for the closure `workspace` builds, and for `export`. A
/// reference inside code is not one, and neither is an escaped
/// `\[[x]]`.
pub fn prose_refs(body: &str) -> Vec<ProseRef> {
    scan_prose(body).0
}

fn scan_prose(body: &str) -> (Vec<ProseRef>, Vec<String>) {
    use pulldown_cmark::{Event, LinkType, Options, Parser, Tag};

    let mut refs = Vec::new();
    let mut links = Vec::new();
    // A code span and a code block carry no link events, so a reference
    // quoted as an example is never read as one.
    for (event, range) in Parser::new_ext(body, Options::ENABLE_WIKILINKS).into_offset_iter() {
        // `![[id]]` arrives as an image; it names a page all the same.
        let (link_type, dest_url, embed) = match event {
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                ..
            }) => (link_type, dest_url, false),
            Event::Start(Tag::Image {
                link_type: link_type @ LinkType::WikiLink { .. },
                dest_url,
                ..
            }) => (link_type, dest_url, true),
            _ => continue,
        };
        match link_type {
            LinkType::WikiLink { has_pothole } => {
                let text = has_pothole
                    .then(|| {
                        body[range.clone()]
                            .trim_start_matches('!')
                            .trim_start_matches("[[")
                            .trim_end_matches("]]")
                            .split_once('|')
                            .map(|(_, text)| text.to_string())
                    })
                    .flatten();
                refs.push(ProseRef {
                    range,
                    target: dest_url.to_string(),
                    text,
                    embed,
                });
            }
            // An autolink is a url by definition.
            LinkType::Autolink | LinkType::Email => {}
            _ => {
                if let Some(target) = relative_page(&dest_url) {
                    links.push(target);
                }
            }
        }
    }
    (refs, links)
}

/// Every `[[reference]]` a page's prose names.
///
/// `workspace` builds the closure from these, and it must see what a
/// reader sees: a reference inside a code span is an example, and the
/// closure does not follow it.
pub fn body_refs(body: &str) -> Vec<String> {
    prose_targets(body).0
}

/// A link destination that names a page file beside this one.
///
/// The title a destination may carry is pulldown-cmark's to strip, so
/// this sees the url alone.
fn relative_page(dest: &str) -> Option<String> {
    let target = dest.trim();
    if target.contains("://") || target.starts_with('#') || !target.ends_with(".md") {
        return None;
    }
    Some(target.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(id: &str, kind: Kind, title: &str, body: &str) -> Page {
        Page {
            id: id.to_string(),
            frontmatter: PageFrontmatter {
                title: title.to_string(),
                description: String::new(),
                kind,
                tags: Vec::new(),
            },
            body: body.to_string(),
            fence: mdstore::Fence::Comment,
        }
    }

    #[test]
    fn a_set_that_did_not_load_is_a_fault() {
        // This check read the labels of unavailable sets and filtered
        // them for a `:` they do not carry, so it returned nothing on
        // every input and hid the condition it exists to report.
        // Nothing tested it, so restoring that filter left every suite
        // green.
        let s = Scratch::new("unavailable", &[]);
        std::fs::write(
            s.0.join("stores.yml"),
            "stores:\n  - alias: absent\n    path: ../no-such-set\n",
        )
        .expect("declare a set that is not there");
        let found = unavailable_sets(&s.0).expect("check ran");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].message.contains("absent"), "{:?}", found[0]);
        assert!(matches!(found[0].severity, Severity::Fault));
    }

    #[test]
    fn a_set_with_no_declarations_reports_nothing() {
        let s = Scratch::new("no-decls", &[]);
        let found = unavailable_sets(&s.0).expect("check ran");
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn an_email_autolink_is_not_a_page_link() {
        // pulldown-cmark reads `<a@b.md>` as an autolink, and its
        // destination ends in `.md`. Without the autolink guard the
        // check reports a fault on an address.
        let (_, links) = prose_targets("write to <someone@example.md> about it");
        assert!(links.is_empty(), "an address read as a link: {links:?}");
    }

    #[test]
    fn a_bare_url_autolink_is_not_a_page_link() {
        let (_, links) = prose_targets("see <https://example.com/page.md>");
        assert!(links.is_empty(), "a url read as a page: {links:?}");
    }

    /// A set on disk holding the pages given, removed when dropped.
    struct Scratch(camino::Utf8PathBuf);

    impl Scratch {
        fn new(name: &str, pages: &[Page]) -> Self {
            let dir = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
                .expect("utf-8 temp dir")
                .join(format!("diataxis-check-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("scratch dir");
            Repo::init(&dir).expect("init");
            let repo = Repo::open(&dir).expect("open");
            for p in pages {
                let path = repo.path_of(&p.id).expect("plain id");
                let text = mdstore::serialize_with(
                    &mdstore::Document {
                        frontmatter: p.frontmatter.clone(),
                        body: p.body.clone(),
                    },
                    mdstore::Fence::Yaml,
                )
                .expect("serialize");
                std::fs::write(&path, text).expect("write page");
            }
            Self(dir)
        }

        fn repo(&self) -> Repo {
            Repo::open(&self.0).expect("open")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_reference_naming_no_page_is_a_fault() {
        // Disabling this check left every suite green, so a set full of
        // dead references reported clean.
        let pages = vec![page("one", Kind::Explanation, "One", "see [[absent]]")];
        let s = Scratch::new("deadref", &pages);
        let found = broken_links(&s.repo(), &pages).expect("check ran");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].message.contains("absent"), "{:?}", found[0]);
        assert!(matches!(found[0].severity, Severity::Fault));
    }

    #[test]
    fn a_markdown_link_naming_no_file_is_a_fault() {
        let pages = vec![page("one", Kind::Explanation, "One", "[x](gone.md)")];
        let s = Scratch::new("deadlink", &pages);
        let found = broken_links(&s.repo(), &pages).expect("check ran");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].message.contains("gone.md"), "{:?}", found[0]);
    }

    #[test]
    fn a_reference_that_resolves_is_no_fault() {
        let pages = vec![
            page("one", Kind::Explanation, "One", "see [[two]]"),
            page("two", Kind::Explanation, "Two", "body"),
        ];
        let s = Scratch::new("liveref", &pages);
        let found = broken_links(&s.repo(), &pages).expect("check ran");
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_how_to_title_that_states_no_goal_is_a_fault() {
        let pages = vec![page("a", Kind::HowTo, "Integrating monitoring", "")];
        let findings = how_to_titles(&pages);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Fault);
    }

    #[test]
    fn a_how_to_title_that_states_a_goal_passes() {
        let pages = vec![page("a", Kind::HowTo, "How to integrate monitoring", "")];
        assert!(how_to_titles(&pages).is_empty());
    }

    #[test]
    fn a_title_check_applies_to_how_to_pages_only() {
        let pages = vec![page("a", Kind::Reference, "Command reference", "")];
        assert!(how_to_titles(&pages).is_empty());
    }

    #[test]
    fn a_type_with_no_pages_is_advice() {
        let pages = vec![page("a", Kind::Tutorial, "Getting started", "")];
        let findings = missing_types(&pages);
        assert_eq!(findings.len(), 3);
        assert!(findings.iter().all(|f| f.severity == Severity::Advice));
    }

    fn names(body: &str) -> Vec<String> {
        prose_refs(body).into_iter().map(|r| r.target).collect()
    }

    #[test]
    fn every_reference_in_prose_is_read_and_an_unclosed_one_is_not() {
        assert_eq!(names("see [[one]] and [[two]]"), ["one", "two"]);
        assert!(names("see [[open").is_empty());
        assert_eq!(names("[[shared:onboarding]]"), ["shared:onboarding"]);
    }

    #[test]
    fn an_escaped_opening_bracket_is_not_a_reference() {
        assert!(prose_refs("Escaped \\[[install]] here.").is_empty());
    }

    #[test]
    fn an_escaped_backslash_before_a_reference_leaves_it_a_reference() {
        // `\\` is one literal backslash; the brackets after it still open
        // a reference, so a broken one is still caught.
        assert_eq!(names("A path \\\\[[install]] here."), ["install"]);
    }

    #[test]
    fn a_link_definition_of_the_same_name_does_not_hide_a_reference() {
        let body = "See [[install]] here.\n\n[install]: https://example.com\n";
        assert_eq!(names(body), ["install"]);
    }

    #[test]
    fn the_text_after_a_pipe_is_kept() {
        let refs = prose_refs("See [[install|the install page]].");
        assert_eq!(refs[0].target, "install");
        assert_eq!(refs[0].text.as_deref(), Some("the install page"));
    }

    #[test]
    fn a_prose_reference_carries_its_byte_range_in_the_body() {
        let body = "Intro.\n\nSee [[install]] and `[[code]]`.\n";
        let refs = prose_refs(body);
        assert_eq!(refs.len(), 1, "{refs:?}");
        assert_eq!(&body[refs[0].range.clone()], "[[install]]");
        assert_eq!(refs[0].target, "install");
    }

    #[test]
    fn a_reference_inside_backticks_is_not_a_link() {
        let body = "A note declares a link inline in its body (`[[note-id]]`).";
        assert!(body_refs(body).is_empty());
    }

    #[test]
    fn a_reference_inside_a_fenced_block_is_not_a_link() {
        let body = "before\n\n```\nsee [[other]] and [x](y.md)\n```\n\nafter";
        let (wiki, links) = prose_targets(body);
        assert!(wiki.is_empty(), "{wiki:?}");
        assert!(links.is_empty(), "{links:?}");
    }

    #[test]
    fn a_reference_in_prose_survives() {
        let body = "see [[other]] and `code` and [x](y.md)";
        let (wiki, links) = prose_targets(body);
        assert_eq!(wiki, ["other"]);
        assert_eq!(links, ["y.md"]);
    }

    #[test]
    fn an_unclosed_backtick_run_is_prose() {
        // A lone backtick opens nothing, so a reference after it is a
        // reference. The stripper this replaced had its own rule for
        // that; CommonMark already says it.
        assert_eq!(body_refs("a ` [[one]]"), ["one"]);
        assert!(body_refs("a ``[[two]]`` c").is_empty());
    }

    #[test]
    fn body_refs_reads_prose_and_not_code() {
        let body = "see [[one]] and `[[two]]`";
        assert_eq!(body_refs(body), ["one"]);
    }

    #[test]
    fn an_indented_code_block_is_not_prose() {
        // Four spaces open a code block in CommonMark. The scanner this
        // replaced read the line as prose and reported a fault on a
        // page that was correct.
        let body = "text\n\n    [[ghost-indented]]\n    [a](gone-indented.md)\n";
        let (wiki, links) = prose_targets(body);
        assert!(wiki.is_empty(), "indented code gave {wiki:?}");
        assert!(links.is_empty(), "indented code gave {links:?}");
    }

    #[test]
    fn a_fenced_block_inside_a_blockquote_is_not_prose() {
        // The scanner this replaced matched a fence only at the start
        // of a line, so a quoted fence never opened and its body read
        // as prose.
        let body = "> ```\n> [[ghost-blockquote]]\n> ```\n";
        let (wiki, _) = prose_targets(body);
        assert!(wiki.is_empty(), "a quoted fence gave {wiki:?}");
    }

    #[test]
    fn a_link_with_a_title_is_still_a_link() {
        // The scanner this replaced required the raw span to end in
        // `.md`, so a title after the url hid a genuinely broken link.
        let (_, links) = prose_targets("[y](also-gone.md \"the title\")");
        assert_eq!(links, ["also-gone.md"]);
    }

    #[test]
    fn a_url_and_an_anchor_are_not_pages() {
        let (_, links) =
            prose_targets("[a](other.md) [b](https://example.com/x.md) [c](#anchor) [d](img.png)");
        assert_eq!(links, ["other.md"]);
    }
}

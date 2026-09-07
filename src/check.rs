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
        let prose = prose_only(&page.body);
        for target in wiki_targets(&prose) {
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
        for target in markdown_targets(&prose) {
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

/// The body with every code span and fenced block removed.
///
/// A link check reads prose only. Documentation quotes syntax
/// constantly, and `[[note-id]]` inside backticks is an example of a
/// reference rather than a reference. Scanning code produced a fault on
/// a page that was correct, which is the one thing a check must not do.
fn prose_only(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut fenced = false;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fenced = !fenced;
            out.push('\n');
            continue;
        }
        if fenced {
            out.push('\n');
            continue;
        }
        out.push_str(&strip_code_spans(line));
        out.push('\n');
    }
    out
}

/// One line with its inline code spans removed.
///
/// A span opens on a run of backticks and closes on a run of the same
/// length. An unclosed run is text, so the rest of the line is kept.
fn strip_code_spans(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(open) = rest.find('`') {
        out.push_str(&rest[..open]);
        let after = &rest[open..];
        let ticks = after.len() - after.trim_start_matches('`').len();
        let fence = &after[..ticks];
        let body = &after[ticks..];
        match body.find(fence) {
            Some(close) => rest = &body[close + ticks..],
            None => {
                out.push_str(after);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Every `[[reference]]` a page body holds, code excluded.
///
/// The store graph needs the same list the link check reads, so both
/// take it from here and cannot disagree about what a reference is.
#[must_use]
pub fn body_refs(body: &str) -> Vec<String> {
    wiki_targets(&prose_only(body))
}

/// Every `[[target]]` in a body.
fn wiki_targets(body: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find("[[") {
        let from_open = &rest[start + 2..];
        let Some(end) = from_open.find("]]") else {
            break;
        };
        let target = &from_open[..end];
        if !target.is_empty() && !target.contains(char::is_whitespace) {
            targets.push(target.to_string());
        }
        rest = &from_open[end + 2..];
    }
    targets
}

/// Every relative `.md` link target in a body.
fn markdown_targets(body: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut rest = body;
    while let Some(open) = rest.find("](") {
        let from_open = &rest[open + 2..];
        let Some(close) = from_open.find(')') else {
            break;
        };
        let target = from_open[..close].trim();
        let is_relative = !target.contains("://") && !target.starts_with('#');
        if is_relative && target.ends_with(".md") {
            targets.push(target.to_string());
        }
        rest = &from_open[close + 1..];
    }
    targets
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

    #[test]
    fn wiki_targets_ignores_an_unclosed_marker() {
        assert_eq!(wiki_targets("see [[one]] and [[two]]"), ["one", "two"]);
        assert_eq!(wiki_targets("see [[open").len(), 0);
        assert_eq!(wiki_targets("[[two words]]").len(), 0);
    }

    #[test]
    fn markdown_targets_keeps_relative_pages_only() {
        let body = "[a](other.md) [b](https://example.com/x.md) [c](#anchor) [d](img.png)";
        assert_eq!(markdown_targets(body), ["other.md"]);
    }

    #[test]
    fn a_reference_inside_backticks_is_not_a_link() {
        let body = "A note declares a link inline in its body (`[[note-id]]`).";
        assert!(wiki_targets(&prose_only(body)).is_empty());
    }

    #[test]
    fn a_reference_inside_a_fenced_block_is_not_a_link() {
        let body = "before\n\n```\nsee [[other]] and [x](y.md)\n```\n\nafter";
        let prose = prose_only(body);
        assert!(wiki_targets(&prose).is_empty());
        assert!(markdown_targets(&prose).is_empty());
    }

    #[test]
    fn a_reference_in_prose_survives_the_strip() {
        let body = "see [[other]] and `code` and [x](y.md)";
        let prose = prose_only(body);
        assert_eq!(wiki_targets(&prose), ["other"]);
        assert_eq!(markdown_targets(&prose), ["y.md"]);
    }

    #[test]
    fn an_unclosed_backtick_run_keeps_the_rest_of_the_line() {
        assert_eq!(strip_code_spans("a `b c"), "a `b c");
        assert_eq!(strip_code_spans("a ``b`` c"), "a  c");
    }

    #[test]
    fn body_refs_reads_prose_and_not_code() {
        let body = "see [[one]]\n\n```\n[[two]]\n```\n\nand `[[three]]`";
        assert_eq!(body_refs(body), ["one"]);
    }
}

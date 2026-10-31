//! Export a set as front-matter Markdown for a static site generator.
//!
//! Each page becomes one file with `---` front matter: `title`,
//! `description`, `weight`, and `params.diataxis_type`. A `[[page]]`
//! reference becomes a relative `page.md` link; a reference in code
//! stays as written. The leading H1 is dropped, because the generator
//! renders the title from the front matter.
//!
//! Export decides nothing a site decides: how pages group in a sidebar,
//! where an "edit this page" link points, which file names the
//! generator reserves. The site's own configuration and templates own
//! those.
//!
//! `check` owns validity, and export runs it first. Export adds one
//! refusal of its own: a reference to a page in another set, or an
//! embed, `![[x]]`, neither of which a single-set export can turn into
//! a link.

use std::collections::HashMap;

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;

use crate::check::{Severity, check, prose_links, prose_refs};
use crate::error::{Error, Result};
use crate::page::{Kind, Page};
use crate::repo::Repo;

/// Write every page of the set rooted at `root` into `out`, a directory
/// that must not exist yet, and return the files written.
///
/// Fails, leaving no directory behind, when `check` reports a fault, a
/// page references or embeds a page export cannot link, `out` exists,
/// or a write fails. Every fault of a kind is named at once.
pub fn export(root: &Utf8Path, out: &Utf8Path) -> Result<Vec<Utf8PathBuf>> {
    if out.as_str().trim().is_empty() {
        return Err(Error::General(
            "--out takes a path for a new directory, not an empty value".to_string(),
        ));
    }
    if out.exists() {
        // The set's own directories get a refusal that never suggests
        // removing them.
        let docs = Repo::open(root)?.docs_dir()?;
        let own = |p: &Utf8Path| p.canonicalize_utf8().ok();
        let out_real = own(out);
        if out_real.is_some() && (out_real == own(&docs) || out_real == own(root)) {
            return Err(Error::General(format!(
                "{out} already exists and holds this set; name a new path outside it"
            )));
        }
        return Err(Error::General(format!(
            "{out} already exists; export creates its own directory. Remove the previous export, or name a path that does not exist yet"
        )));
    }
    let faults: Vec<String> = check(root)?
        .into_iter()
        .filter(|f| f.severity == Severity::Fault)
        .map(|f| format!("  {}: {}", f.subject, f.message))
        .collect();
    if !faults.is_empty() {
        return Err(Error::General(format!(
            "export writes a set with no `diataxis check` faults, and this set has {}:\n{}",
            faults.len(),
            faults.join("\n")
        )));
    }
    let pages = Repo::open(root)?.list_strict()?;
    let titles: HashMap<&str, &str> = pages.iter().map(|p| (p.id.as_str(), p.title())).collect();

    let mut ordered: Vec<&Page> = pages.iter().collect();
    ordered.sort_by_key(|p| {
        let kind = Kind::ALL.iter().position(|k| *k == p.kind());
        (kind, p.title().to_lowercase(), p.id.clone())
    });
    let mut files = Vec::with_capacity(ordered.len());
    let mut refused = Vec::new();
    for (position, page) in ordered.into_iter().enumerate() {
        // From 1, because Hugo reads a weight of zero as no weight. A
        // set of more than u32::MAX pages does not fit on a disk.
        let weight = u32::try_from(position + 1).unwrap_or(u32::MAX);
        match render_page(page, &titles, weight) {
            Ok(text) => files.push((format!("{}.md", page.id), text)),
            Err(problems) => refused.extend(problems),
        }
    }
    if !refused.is_empty() {
        return Err(Error::General(format!(
            "export cannot link {} reference(s) or link(s):\n{}",
            refused.len(),
            refused.join("\n")
        )));
    }

    std::fs::create_dir(out).map_err(|e| {
        let parent_missing = out
            .parent()
            .is_some_and(|p| !p.as_str().is_empty() && !p.exists());
        let hint = if parent_missing {
            "; export makes the directory itself, not its parents, so the parent must exist"
        } else {
            ""
        };
        Error::General(format!("cannot create {out}: {e}{hint}"))
    })?;
    write_files(out, files, |path, text| std::fs::write(path, text))
}

/// Write each file into `out`, a directory export has just made. When a
/// write fails, `out` is removed again, so the path is as it was.
fn write_files(
    out: &Utf8Path,
    files: Vec<(String, String)>,
    write: impl Fn(&Utf8Path, &str) -> std::io::Result<()>,
) -> Result<Vec<Utf8PathBuf>> {
    let mut written = Vec::with_capacity(files.len());
    for (name, text) in files {
        let path = out.join(name);
        if let Err(e) = write(&path, &text) {
            // The directory is export's own, made above, so removing it
            // leaves the path as it was.
            let cleanup = match std::fs::remove_dir_all(out) {
                Ok(()) => String::new(),
                Err(c) => {
                    format!("; removing {out} also failed ({c}), so remove it before a new export")
                }
            };
            return Err(Error::General(format!("cannot write {path}: {e}{cleanup}")));
        }
        written.push(path);
    }
    Ok(written)
}

#[derive(Serialize)]
struct PageFront<'a> {
    title: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    description: &'a str,
    weight: u32,
    params: PageParams,
}

#[derive(Serialize)]
struct PageParams {
    /// Not `type`: Hugo reserves that key for the layout it picks.
    diataxis_type: &'static str,
}

/// One page as a front-matter Markdown file, ending in one newline, or
/// every reference on it that export cannot link.
fn render_page(
    page: &Page,
    titles: &HashMap<&str, &str>,
    weight: u32,
) -> std::result::Result<String, Vec<String>> {
    let front = PageFront {
        title: page.title(),
        description: &page.frontmatter.description,
        weight,
        params: PageParams {
            diataxis_type: page.kind().as_str(),
        },
    };
    let body = rewrite_body(&page.id, &page.body, titles)?;
    let front = yaml_serde::to_string(&front)
        .map_err(|e| vec![format!("  {}: front matter: {e}", page.id)])?;
    Ok(format!("---\n{front}---\n\n{}\n", body.trim_end()))
}

/// The body with its references rewritten and its leading H1 dropped,
/// or every reference on the page that export cannot link.
fn rewrite_body(
    id: &str,
    body: &str,
    titles: &HashMap<&str, &str>,
) -> std::result::Result<String, Vec<String>> {
    use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag};

    let mut start = 0;
    if let Some((
        Event::Start(Tag::Heading {
            level: HeadingLevel::H1,
            ..
        }),
        range,
    )) = Parser::new_ext(body, Options::empty())
        .into_offset_iter()
        .next()
    {
        start = range.end;
    }
    let rest = &body[start..];
    let mut cursor = start + (rest.len() - rest.trim_start_matches(['\n', '\r']).len());

    // Byte ranges from check's own scan, not a re-serialised document,
    // so every other byte of the page survives.
    let mut out = String::with_capacity(body.len());
    let mut problems = Vec::new();
    for reference in prose_refs(body) {
        if reference.range.start < cursor {
            continue;
        }
        let target = reference.target.as_str();
        if reference.embed {
            problems.push(format!(
                "  {id}: ![[{target}]] embeds a page, which a site cannot render; write [[{target}]] to link it"
            ));
            continue;
        }
        if target.contains(':') {
            problems.push(format!(
                "  {id}: [[{target}]] names a page in another set; write the other site's URL instead"
            ));
            continue;
        }
        let Some(title) = titles.get(target) else {
            problems.push(format!(
                "  {id}: [[{target}]] names no page; create the page or correct the reference"
            ));
            continue;
        };
        let text = match &reference.text {
            Some(text) => text.clone(),
            None => escape_link_text(title),
        };
        out.push_str(&body[cursor..reference.range.start]);
        // Angle brackets let the destination hold a space or a lone
        // parenthesis, which a page id may carry; `#`, `?` and `%` are
        // encoded, because a link reads them as a fragment or a query.
        out.push_str(&format!("[{text}](<{}.md>)", encode_destination(target)));
        cursor = reference.range.end;
    }
    // A relative `.md` link travels to the site as written, so it must
    // name a page of this set, the only files export writes beside it.
    for link in prose_links(&body[start..]) {
        let stem = link.strip_prefix("./").unwrap_or(&link);
        let is_page = stem
            .strip_suffix(".md")
            .is_some_and(|id| !id.contains('/') && titles.contains_key(id));
        if !is_page {
            problems.push(format!(
                "  {id}: the link to {link} names a file export does not write; write [[id]] for a page of this set, or the file's full URL"
            ));
        }
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    out.push_str(&body[cursor..]);
    Ok(out)
}

/// A page id as a link destination: the characters a URL reads as
/// structure are percent-encoded, so the link names the file.
fn encode_destination(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    for c in id.chars() {
        match c {
            '%' => out.push_str("%25"),
            '#' => out.push_str("%23"),
            '?' => out.push_str("%3F"),
            _ => out.push(c),
        }
    }
    out
}

/// A title as literal link text: every character CommonMark could read
/// as markup is escaped, so the reader sees the title as written.
fn escape_link_text(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    for c in title.chars() {
        if matches!(
            c,
            '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>' | '!' | '&'
        ) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::PageFrontmatter;

    fn titles() -> HashMap<&'static str, &'static str> {
        HashMap::from([
            ("install", "Install acme"),
            ("why", "Why acme [queues]"),
            ("marked", "Install *acme* \\"),
        ])
    }

    fn page(id: &str, kind: Kind, title: &str, body: &str) -> Page {
        Page {
            id: id.to_string(),
            frontmatter: PageFrontmatter {
                title: title.to_string(),
                description: "One line.".to_string(),
                kind,
                tags: vec![],
            },
            body: body.to_string(),
            fence: mdstore::Fence::Comment,
        }
    }

    #[test]
    fn a_reference_in_prose_becomes_a_relative_link_with_the_title() {
        let out = rewrite_body("a", "See [[install]] first.\n", &titles()).unwrap();
        assert_eq!(out, "See [Install acme](<install.md>) first.\n");
    }

    #[test]
    fn a_reference_with_its_own_text_keeps_the_text() {
        let out = rewrite_body("a", "See [[install|the install]].\n", &titles()).unwrap();
        assert_eq!(out, "See [the install](<install.md>).\n");
    }

    #[test]
    fn markup_in_a_title_is_escaped_in_the_link_text() {
        let out = rewrite_body("a", "See [[why]] and [[marked]].\n", &titles()).unwrap();
        assert_eq!(
            out,
            "See [Why acme \\[queues\\]](<why.md>) and [Install \\*acme\\* \\\\](<marked.md>).\n"
        );
    }

    #[test]
    fn a_relative_link_to_a_file_export_does_not_write_is_refused() {
        for body in [
            "See [the readme](../README.md).\n",
            "See [sub](sub/x.md).\n",
            "See [gone](absent.md).\n",
            "See [the readme](../README.md#top).\n",
            "See [gone](absent.md?v=1).\n",
        ] {
            let err = rewrite_body("a", body, &titles()).unwrap_err();
            assert_eq!(err.len(), 1, "{body}: {err:?}");
            assert!(err[0].contains("full URL"), "{err:?}");
        }
        // A link to a page of this set, with or without ./, passes.
        let body = "See [install](install.md#steps) and [why](./why.md).\n";
        assert_eq!(rewrite_body("a", body, &titles()).unwrap(), body);
    }

    #[test]
    fn an_id_with_a_hash_question_mark_or_percent_is_encoded() {
        let titles = HashMap::from([("c#", "C sharp"), ("why?", "Why"), ("100%", "All")]);
        let out = rewrite_body("a", "[[c#]] [[why?]] [[100%]]\n", &titles).unwrap();
        assert_eq!(
            out,
            "[C sharp](<c%23.md>) [Why](<why%3F.md>) [All](<100%25.md>)\n"
        );
    }

    #[test]
    fn an_id_with_a_space_or_a_lone_parenthesis_still_makes_a_link() {
        let titles = HashMap::from([("two words", "Two words"), ("half(", "Half")]);
        let out = rewrite_body("a", "See [[two words]] and [[half(]].\n", &titles).unwrap();
        // Parse the output: each must be a link to its own file.
        let dests: Vec<String> = pulldown_cmark::Parser::new(&out)
            .filter_map(|e| match e {
                pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link { dest_url, .. }) => {
                    Some(dest_url.to_string())
                }
                _ => None,
            })
            .collect();
        assert_eq!(dests, ["two words.md", "half(.md"], "{out}");
    }

    #[test]
    fn an_embed_is_refused_with_its_fix() {
        let err = rewrite_body("a", "See ![[install]].\n", &titles()).unwrap_err();
        assert_eq!(err.len(), 1, "{err:?}");
        assert!(err[0].contains("![[install]]") && err[0].contains("write [[install]]"));
    }

    #[test]
    fn every_unlinkable_reference_on_a_page_is_named() {
        let body = "See [[tisket:a]], [[zettel:b]] and ![[install]].\n";
        let err = rewrite_body("a", body, &titles()).unwrap_err();
        assert_eq!(err.len(), 3, "{err:?}");
    }

    #[test]
    fn a_reference_in_code_or_escaped_stays_as_written() {
        for body in [
            "Write `[[install]]` in a body.\n",
            "before\n\n```\nsee [[install]]\n```\n\n    and [[install]]\n",
            "Escaped \\[[install]] here.\n",
        ] {
            assert_eq!(rewrite_body("a", body, &titles()).unwrap(), body);
        }
    }

    #[test]
    fn a_reference_in_the_dropped_h1_goes_with_it_and_later_ones_are_rewritten() {
        // The H1 is dropped whole, so a reference in it leaves no text
        // behind; `check` has already resolved it before export runs.
        let body = "# Using [[absent]]\n\nSee [[install]].\n";
        assert_eq!(
            rewrite_body("a", body, &titles()).unwrap(),
            "See [Install acme](<install.md>).\n"
        );
    }

    #[test]
    fn a_reference_that_names_no_page_is_an_error() {
        let err = rewrite_body("a", "See [[absent]].\n", &titles()).unwrap_err();
        assert!(err[0].contains("a: [[absent]]"), "{err:?}");
        assert!(err[0].contains("create the page"), "{err:?}");
    }

    #[test]
    fn a_reference_to_another_set_is_an_error() {
        let err = rewrite_body("a", "See [[tisket:workflow]].\n", &titles()).unwrap_err();
        assert!(err[0].contains("[[tisket:workflow]]"), "{err:?}");
        assert!(err[0].contains("URL"), "{err:?}");
    }

    #[test]
    fn the_leading_h1_is_dropped_and_a_later_one_kept() {
        let body = "\n# Install acme\n\nText.\n\n# Another\n";
        assert_eq!(
            rewrite_body("a", body, &titles()).unwrap(),
            "Text.\n\n# Another\n"
        );
        let body = "Text first.\n\n# Heading\n";
        assert_eq!(rewrite_body("a", body, &titles()).unwrap(), body);
        // Only a leading H1 is the title; a body that opens with an H2
        // keeps it.
        let body = "## Setup\n\nText.\n";
        assert_eq!(rewrite_body("a", body, &titles()).unwrap(), body);
    }

    #[test]
    fn every_character_markdown_or_html_reads_is_escaped_in_link_text() {
        assert_eq!(
            escape_link_text(r"\`*_[]<>!& plain"),
            r"\\\`\*\_\[\]\<\>\!\& plain"
        );
    }

    #[test]
    fn front_matter_carries_title_description_weight_and_type_only() {
        let p = page(
            "install",
            Kind::HowTo,
            "How to install: \"acme\"",
            "# How to install\n\nBody.",
        );
        let out = render_page(&p, &titles(), 3).unwrap();
        let (front, body) = split_front(&out);
        let yaml: yaml_serde::Value = yaml_serde::from_str(front).unwrap();
        assert_eq!(yaml["title"].as_str(), Some("How to install: \"acme\""));
        assert_eq!(yaml["description"].as_str(), Some("One line."));
        assert_eq!(yaml["weight"].as_u64(), Some(3));
        assert_eq!(yaml["params"]["diataxis_type"].as_str(), Some("how-to"));
        // Hugo reserves `type` for the layout, and an edit link is the
        // site's to configure.
        assert!(yaml.get("type").is_none());
        assert!(yaml.get("editURL").is_none());
        assert_eq!(body, "Body.\n");
    }

    #[test]
    fn an_empty_description_writes_no_description_key() {
        let mut p = page("install", Kind::HowTo, "How to install", "Body.\n");
        p.frontmatter.description = String::new();
        let out = render_page(&p, &titles(), 1).unwrap();
        let yaml: yaml_serde::Value = yaml_serde::from_str(split_front(&out).0).unwrap();
        assert!(yaml.get("description").is_none(), "{out}");
    }

    fn split_front(text: &str) -> (&str, &str) {
        let rest = text.strip_prefix("---\n").expect("opens with ---");
        let end = rest.find("\n---\n").expect("closes with ---");
        (&rest[..end], rest[end + 5..].trim_start_matches('\n'))
    }

    /// A set in a fresh directory, removed when the test ends.
    struct Scratch(Utf8PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir = Utf8PathBuf::from_path_buf(std::env::temp_dir())
                .expect("utf-8 temp dir")
                .join(format!("diataxis-export-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(dir.join("docs")).expect("scratch dir");
            std::fs::write(dir.join("diataxis.yml"), "docs_dir: docs\n").expect("config");
            Self(dir)
        }

        fn page(&self, id: &str, text: &str) {
            std::fs::write(self.0.join("docs").join(format!("{id}.md")), text).expect("page");
        }

        fn export(&self, out: &Utf8Path) -> Result<Vec<Utf8PathBuf>> {
            export(&self.0, out)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn meta(title: &str, kind: &str) -> String {
        format!(
            "<!-- metadata\ntitle: {title}\ndescription: d\ntype: {kind}\n-->\n\n# {title}\n\nBody.\n"
        )
    }

    #[test]
    fn export_writes_one_file_per_page_in_diataxis_then_title_order() {
        let s = Scratch::new("set");
        s.page("zeta-start", &meta("Zeta start", "tutorial"));
        s.page("alpha-start", &meta("Alpha start", "tutorial"));
        s.page("flags", &meta("Flags", "reference"));
        s.page("how-to-run", &meta("How to run it", "guide"));
        let out = s.0.join("out");
        let mut written = s.export(&out).unwrap();
        written.sort();
        let names: Vec<_> = written.iter().filter_map(|p| p.file_name()).collect();
        assert_eq!(
            names,
            [
                "alpha-start.md",
                "flags.md",
                "how-to-run.md",
                "zeta-start.md"
            ]
        );
        let weight = |id: &str| {
            let text = std::fs::read_to_string(out.join(format!("{id}.md"))).unwrap();
            let yaml: yaml_serde::Value = yaml_serde::from_str(split_front(&text).0).unwrap();
            yaml["weight"].as_u64().unwrap()
        };
        // Tutorials, then how-to guides, then reference; by title inside
        // a type; from 1, because Hugo reads zero as no weight.
        assert_eq!(weight("alpha-start"), 1);
        assert_eq!(weight("zeta-start"), 2);
        assert_eq!(weight("how-to-run"), 3);
        assert_eq!(weight("flags"), 4);
    }

    #[test]
    fn titles_order_without_regard_to_case() {
        let s = Scratch::new("case");
        s.page("b", &meta("beta", "tutorial"));
        s.page("a", &meta("Alpha", "tutorial"));
        s.page("c", &meta("Gamma", "tutorial"));
        let out = s.0.join("out");
        s.export(&out).unwrap();
        let weight = |id: &str| {
            let text = std::fs::read_to_string(out.join(format!("{id}.md"))).unwrap();
            let yaml: yaml_serde::Value = yaml_serde::from_str(split_front(&text).0).unwrap();
            yaml["weight"].as_u64().unwrap()
        };
        assert_eq!((weight("a"), weight("b"), weight("c")), (1, 2, 3));
    }

    #[test]
    fn export_reads_the_yaml_metadata_form_too() {
        let s = Scratch::new("forms");
        s.page(
            "new",
            "---\ntitle: New\ndescription: d\ntype: explanation\n---\n\nBody.\n",
        );
        let out = s.0.join("out");
        s.export(&out).unwrap();
        let text = std::fs::read_to_string(out.join("new.md")).unwrap();
        assert!(text.contains("diataxis_type: explanation"), "{text}");
    }

    #[test]
    fn a_set_with_a_check_fault_is_refused_and_nothing_is_created() {
        let s = Scratch::new("fault");
        s.page("good", &meta("Good", "tutorial"));
        s.page(
            "bad",
            "<!-- metadata\ntitle: Bad\ntype: reference\n-->\n\nSee [[absent]].\n",
        );
        let out = s.0.join("out");
        let err = s.export(&out).unwrap_err();
        assert!(err.to_string().contains("diataxis check"), "{err}");
        // The faults are named, so no second command is needed.
        assert!(err.to_string().contains("[[absent]]"), "{err}");
        assert!(!out.exists(), "a refused export created {out}");
    }

    #[test]
    fn an_existing_path_is_refused_and_left_alone() {
        // The pages directory is the dangerous case: exporting into it
        // would overwrite each source page with its exported form.
        let s = Scratch::new("exists");
        s.page("first", &meta("First", "tutorial"));
        let docs = s.0.join("docs");
        let before = std::fs::read_to_string(docs.join("first.md")).unwrap();
        // The set's own directories: refused, and never with advice to
        // remove them.
        for out in [docs.clone(), s.0.clone()] {
            let err = s.export(&out).unwrap_err();
            assert!(err.to_string().contains("holds this set"), "{err}");
            assert!(!err.to_string().contains("emove"), "{err}");
        }
        // Any other existing directory, such as a previous export.
        std::fs::create_dir_all(s.0.join("previous")).unwrap();
        let err = s.export(&s.0.join("previous")).unwrap_err();
        assert!(err.to_string().contains("already exists"), "{err}");
        assert!(
            err.to_string().contains("Remove the previous export"),
            "{err}"
        );
        assert_eq!(
            std::fs::read_to_string(docs.join("first.md")).unwrap(),
            before
        );
    }

    #[test]
    fn a_failed_write_removes_the_directory_export_made() {
        let s = Scratch::new("writefail");
        let out = s.0.join("out");
        std::fs::create_dir(&out).unwrap();
        let files = vec![
            ("first.md".to_string(), "one".to_string()),
            ("second.md".to_string(), "two".to_string()),
        ];
        let err = write_files(&out, files, |path, text| {
            if path.as_str().ends_with("second.md") {
                Err(std::io::Error::other("disk full"))
            } else {
                std::fs::write(path, text)
            }
        })
        .unwrap_err();
        assert!(err.to_string().contains("second.md"), "{err}");
        assert!(!out.exists(), "a failed write left {out}");
    }

    #[test]
    fn a_body_that_opens_with_an_h2_keeps_it() {
        let body = "## Before you begin\n\nText.\n";
        assert_eq!(rewrite_body("a", body, &titles()).unwrap(), body);
    }

    #[test]
    fn an_empty_out_is_refused() {
        let s = Scratch::new("emptyout");
        s.page("first", &meta("First", "tutorial"));
        let err = s.export(Utf8Path::new("")).unwrap_err();
        assert!(err.to_string().contains("empty"), "{err}");
    }

    #[test]
    fn a_cross_set_reference_is_refused_and_nothing_is_created() {
        let s = Scratch::new("crossset");
        s.page(
            "first",
            "<!-- metadata\ntitle: First\ntype: tutorial\n-->\n\nSee [[shared:onboarding]].\n",
        );
        let out = s.0.join("out");
        let err = s.export(&out).unwrap_err();
        assert!(err.to_string().contains("[[shared:onboarding]]"), "{err}");
        assert!(!out.exists());
    }
}

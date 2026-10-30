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
//! refusal of its own: a reference to a page in another set, which a
//! single-set export cannot turn into a link.

use std::collections::HashMap;

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;

use crate::check::{Severity, check, prose_refs};
use crate::error::{Error, Result};
use crate::page::{Kind, Page};
use crate::repo::Repo;

/// Write every page of the set into `out`, a directory that must not
/// exist yet, and return the files written.
///
/// Fails, leaving no directory behind, when `check` reports a fault, a
/// page references a page in another set, `out` exists, or a write
/// fails.
pub fn export(repo: &Repo, root: &Utf8Path, out: &Utf8Path) -> Result<Vec<Utf8PathBuf>> {
    if out.as_str().trim().is_empty() {
        return Err(Error::General(
            "--out takes a path for a new directory, not an empty value".to_string(),
        ));
    }
    if out.exists() {
        return Err(Error::General(format!(
            "{out} already exists; export creates its own directory, so name a path that does not exist yet"
        )));
    }
    let faults = check(root)?
        .into_iter()
        .filter(|f| f.severity == Severity::Fault)
        .count();
    if faults > 0 {
        return Err(Error::General(format!(
            "`diataxis check` reports {faults} fault(s); export writes a set with none. Run `diataxis check` to see them"
        )));
    }
    let pages = repo.list_strict()?;
    let titles: HashMap<&str, &str> = pages.iter().map(|p| (p.id.as_str(), p.title())).collect();

    let mut ordered: Vec<&Page> = pages.iter().collect();
    ordered.sort_by_key(|p| {
        let kind = Kind::ALL.iter().position(|k| *k == p.kind());
        (kind, p.title().to_lowercase(), p.id.clone())
    });
    let mut files = Vec::with_capacity(ordered.len());
    for (position, page) in ordered.into_iter().enumerate() {
        // From 1, because Hugo reads a weight of zero as no weight.
        let weight = u32::try_from(position + 1)
            .map_err(|_| Error::General("too many pages to order".to_string()))?;
        files.push((
            format!("{}.md", page.id),
            render_page(page, &titles, weight)?,
        ));
    }

    std::fs::create_dir(out).map_err(|e| Error::General(format!("cannot create {out}: {e}")))?;
    let mut written = Vec::with_capacity(files.len());
    for (name, text) in files {
        let path = out.join(name);
        if let Err(e) = std::fs::write(&path, text) {
            // The directory is export's own, made above, so removing it
            // leaves the path as it was.
            let _ = std::fs::remove_dir_all(out);
            return Err(Error::General(format!("cannot write {path}: {e}")));
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

/// One page as a front-matter Markdown file, ending in one newline.
fn render_page(page: &Page, titles: &HashMap<&str, &str>, weight: u32) -> Result<String> {
    let front = PageFront {
        title: page.title(),
        description: &page.frontmatter.description,
        weight,
        params: PageParams {
            diataxis_type: page.kind().as_str(),
        },
    };
    let body = rewrite_body(&page.id, &page.body, titles)?;
    Ok(format!(
        "---\n{}---\n\n{}\n",
        yaml_serde::to_string(&front)?,
        body.trim_end()
    ))
}

/// The body with its references rewritten and its leading H1 dropped.
fn rewrite_body(id: &str, body: &str, titles: &HashMap<&str, &str>) -> Result<String> {
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
    for reference in prose_refs(body) {
        if reference.range.start < cursor {
            continue;
        }
        let target = reference.target.as_str();
        if target.contains(':') {
            return Err(Error::General(format!(
                "page {id}: [[{target}]] names a page in another set, which a single-set export cannot link; write the other site's URL instead"
            )));
        }
        let Some(title) = titles.get(target) else {
            return Err(Error::General(format!(
                "page {id}: [[{target}]] names no page; create the page or correct the reference"
            )));
        };
        let text = match &reference.text {
            Some(text) => text.clone(),
            None => escape_link_text(title),
        };
        out.push_str(&body[cursor..reference.range.start]);
        out.push_str(&format!("[{text}]({target}.md)"));
        cursor = reference.range.end;
    }
    out.push_str(&body[cursor..]);
    Ok(out)
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
        assert_eq!(out, "See [Install acme](install.md) first.\n");
    }

    #[test]
    fn a_reference_with_its_own_text_keeps_the_text() {
        let out = rewrite_body("a", "See [[install|the install]].\n", &titles()).unwrap();
        assert_eq!(out, "See [the install](install.md).\n");
    }

    #[test]
    fn markup_in_a_title_is_escaped_in_the_link_text() {
        let out = rewrite_body("a", "See [[why]] and [[marked]].\n", &titles()).unwrap();
        assert_eq!(
            out,
            "See [Why acme \\[queues\\]](why.md) and [Install \\*acme\\* \\\\](marked.md).\n"
        );
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
            "See [Install acme](install.md).\n"
        );
    }

    #[test]
    fn a_reference_that_names_no_page_is_an_error() {
        let err = rewrite_body("a", "See [[absent]].\n", &titles()).unwrap_err();
        assert!(err.to_string().contains("page a: [[absent]]"), "{err}");
        assert!(err.to_string().contains("create the page"), "{err}");
    }

    #[test]
    fn a_reference_to_another_set_is_an_error() {
        let err = rewrite_body("a", "See [[tisket:workflow]].\n", &titles()).unwrap_err();
        assert!(err.to_string().contains("[[tisket:workflow]]"), "{err}");
        assert!(err.to_string().contains("URL"), "{err}");
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
            export(&Repo::open(&self.0).expect("open"), &self.0, out)
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
        for out in [docs.clone(), s.0.clone(), s.0.join("empty")] {
            std::fs::create_dir_all(s.0.join("empty")).unwrap();
            let err = s.export(&out).unwrap_err();
            assert!(err.to_string().contains("already exists"), "{err}");
            assert!(!err.to_string().contains("emove"), "{err}");
        }
        assert_eq!(
            std::fs::read_to_string(docs.join("first.md")).unwrap(),
            before
        );
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

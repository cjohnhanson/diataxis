//! Export a set as content for a static site generator.
//!
//! Each page becomes one Markdown file with `---` front matter, which
//! Hugo and the other front-matter generators read. A `[[page]]`
//! reference in prose becomes a relative `page.md` link. A reference
//! inside a code span or a code block is an example and stays as
//! written. The leading H1 is dropped, because the generator renders
//! the title from the front matter.
//!
//! The sidebar groups pages by Diataxis type. A separator file per
//! type that has pages carries the group label. A type with no pages
//! gets no separator, because Diataxis builds no empty structure.

use std::collections::HashMap;
use std::ops::Range;

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;

use crate::error::{Error, Result};
use crate::page::{Kind, Page};
use crate::repo::Repo;

/// What `export` is given beyond the output directory.
#[derive(Default)]
pub struct ExportOptions<'a> {
    /// The URL a page's source file sits under, for an "edit this page"
    /// link: `https://github.com/acme/tool/edit/main/docs`. The page's
    /// file name is joined to it.
    pub edit_base: Option<&'a str>,
}

/// Write every page of the set into `out`, and return the files written.
///
/// Fails on a page that does not parse, on a reference that names no
/// page, and on a reference to another set. A site built from a partial
/// set reads as complete, so a fault stops the export.
pub fn export(
    repo: &Repo,
    out: &Utf8Path,
    options: &ExportOptions<'_>,
) -> Result<Vec<Utf8PathBuf>> {
    let pages = repo.list_strict()?;
    for page in &pages {
        if Kind::ALL.iter().any(|k| page.id == separator_id(*k)) {
            return Err(Error::General(format!(
                "page {} has the name of a sidebar separator file; rename the page",
                page.id
            )));
        }
    }
    let titles: HashMap<&str, &str> = pages.iter().map(|p| (p.id.as_str(), p.title())).collect();

    std::fs::create_dir_all(out)?;
    let mut written = Vec::new();
    for (index, kind) in Kind::ALL.into_iter().enumerate() {
        let mut group: Vec<&Page> = pages.iter().filter(|p| p.kind() == kind).collect();
        if group.is_empty() {
            continue;
        }
        group.sort_by_key(|p| (p.title().to_lowercase(), p.id.clone()));
        // A thousand weights per group, starting at 1, because Hugo
        // reads a weight of zero as no weight at all.
        let base = u32::try_from(index).unwrap_or(0) * 1000;
        let separator = out.join(format!("{}.md", separator_id(kind)));
        std::fs::write(&separator, render_separator(kind, base + 1)?)?;
        written.push(separator);
        for (position, page) in group.into_iter().enumerate() {
            let weight = base + 2 + u32::try_from(position).unwrap_or(u32::MAX - base - 2);
            let path = out.join(format!("{}.md", page.id));
            std::fs::write(
                &path,
                render_page(page, &titles, weight, options.edit_base)?,
            )?;
            written.push(path);
        }
    }
    Ok(written)
}

/// The sidebar label for one type's group.
#[must_use]
pub fn group_label(kind: Kind) -> &'static str {
    match kind {
        Kind::Tutorial => "Tutorials",
        Kind::HowTo => "How-to guides",
        Kind::Reference => "Reference",
        Kind::Explanation => "Explanation",
    }
}

fn separator_id(kind: Kind) -> String {
    format!("separator-{}", kind.as_str())
}

#[derive(Serialize)]
struct PageFront<'a> {
    title: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    description: &'a str,
    weight: u32,
    params: PageParams,
    #[serde(rename = "editURL", skip_serializing_if = "Option::is_none")]
    edit_url: Option<String>,
}

#[derive(Serialize)]
struct PageParams {
    /// Not `type`: Hugo reserves that key for the layout it picks.
    diataxis_type: &'static str,
}

#[derive(Serialize)]
struct SeparatorFront {
    title: &'static str,
    weight: u32,
    sidebar: SeparatorSidebar,
    build: SeparatorBuild,
}

#[derive(Serialize)]
struct SeparatorSidebar {
    separator: bool,
}

/// A separator is listed in the sidebar and never rendered as a page.
#[derive(Serialize)]
struct SeparatorBuild {
    render: &'static str,
    list: &'static str,
}

fn render_separator(kind: Kind, weight: u32) -> Result<String> {
    let front = SeparatorFront {
        title: group_label(kind),
        weight,
        sidebar: SeparatorSidebar { separator: true },
        build: SeparatorBuild {
            render: "never",
            list: "always",
        },
    };
    Ok(format!("---\n{}---\n", yaml_serde::to_string(&front)?))
}

/// One page as a front-matter Markdown file.
fn render_page(
    page: &Page,
    titles: &HashMap<&str, &str>,
    weight: u32,
    edit_base: Option<&str>,
) -> Result<String> {
    let front = PageFront {
        title: page.title(),
        description: &page.frontmatter.description,
        weight,
        params: PageParams {
            diataxis_type: page.kind().as_str(),
        },
        edit_url: edit_base.map(|base| format!("{}/{}.md", base.trim_end_matches('/'), page.id)),
    };
    let body = rewrite_body(&page.id, &page.body, titles)?;
    Ok(format!(
        "---\n{}---\n\n{body}",
        yaml_serde::to_string(&front)?
    ))
}

/// The body with its references rewritten and its leading H1 dropped.
fn rewrite_body(id: &str, body: &str, titles: &HashMap<&str, &str>) -> Result<String> {
    use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag};

    // What the parser calls code is left alone. Byte ranges, not a
    // re-serialised document, so every other byte of the page survives.
    let mut code: Vec<Range<usize>> = Vec::new();
    let mut start = 0;
    for (index, (event, range)) in Parser::new_ext(body, Options::empty())
        .into_offset_iter()
        .enumerate()
    {
        match event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                ..
            }) if index == 0 => start = range.end,
            Event::Start(Tag::CodeBlock(_))
            | Event::Code(_)
            | Event::Html(_)
            | Event::InlineHtml(_) => code.push(range),
            _ => {}
        }
    }

    let rest = &body[start..];
    let skipped = rest.len() - rest.trim_start_matches(['\n', '\r']).len();
    let mut cursor = start + skipped;
    let mut out = String::with_capacity(body.len());
    while let Some(found) = body[cursor..].find("[[") {
        let open = cursor + found;
        let Some(len) = body[open + 2..].find("]]") else {
            break;
        };
        let target = &body[open + 2..open + 2 + len];
        let close = open + 2 + len + 2;
        let in_code = code.iter().any(|r| r.contains(&open));
        if in_code || target.is_empty() || target.contains(char::is_whitespace) {
            out.push_str(&body[cursor..open + 2]);
            cursor = open + 2;
            continue;
        }
        if target.contains(':') {
            return Err(Error::General(format!(
                "page {id}: [[{target}]] names a page in another set; export resolves references inside one set only"
            )));
        }
        let Some(title) = titles.get(target) else {
            return Err(Error::General(format!(
                "page {id}: [[{target}]] names no page"
            )));
        };
        out.push_str(&body[cursor..open]);
        let text = title.replace('[', "\\[").replace(']', "\\]");
        out.push_str(&format!("[{text}]({target}.md)"));
        cursor = close;
    }
    out.push_str(&body[cursor..]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::PageFrontmatter;

    fn titles() -> HashMap<&'static str, &'static str> {
        HashMap::from([("install", "Install acme"), ("why", "Why acme [queues]")])
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
    fn brackets_in_a_title_are_escaped_in_the_link_text() {
        let out = rewrite_body("a", "See [[why]].\n", &titles()).unwrap();
        assert_eq!(out, "See [Why acme \\[queues\\]](why.md).\n");
    }

    #[test]
    fn a_reference_in_a_code_span_stays_as_written() {
        let body = "Write `[[install]]` in a body.\n";
        assert_eq!(rewrite_body("a", body, &titles()).unwrap(), body);
    }

    #[test]
    fn a_reference_in_a_fenced_or_indented_block_stays_as_written() {
        let body =
            "before\n\n```\nsee [[install]]\n```\n\n    and [[install]]\n\nafter [[install]]\n";
        let out = rewrite_body("a", body, &titles()).unwrap();
        assert_eq!(
            out,
            "before\n\n```\nsee [[install]]\n```\n\n    and [[install]]\n\nafter [Install acme](install.md)\n"
        );
    }

    #[test]
    fn a_reference_that_names_no_page_is_an_error() {
        let err = rewrite_body("a", "See [[absent]].\n", &titles()).unwrap_err();
        assert!(err.to_string().contains("[[absent]]"), "{err}");
        assert!(err.to_string().contains("a"), "{err}");
    }

    #[test]
    fn a_reference_to_another_set_is_an_error() {
        let err = rewrite_body("a", "See [[tisket:workflow]].\n", &titles()).unwrap_err();
        assert!(err.to_string().contains("[[tisket:workflow]]"), "{err}");
    }

    #[test]
    fn the_leading_h1_is_dropped_and_a_later_one_kept() {
        let body = "\n# Install acme\n\nText.\n\n# Another\n";
        assert_eq!(
            rewrite_body("a", body, &titles()).unwrap(),
            "Text.\n\n# Another\n"
        );
    }

    #[test]
    fn a_body_that_opens_with_prose_keeps_it() {
        let body = "Text first.\n\n# Heading\n";
        assert_eq!(rewrite_body("a", body, &titles()).unwrap(), body);
    }

    #[test]
    fn a_relative_md_link_is_left_for_the_generator() {
        let body = "See [install](install.md).\n";
        assert_eq!(rewrite_body("a", body, &titles()).unwrap(), body);
    }

    #[test]
    fn front_matter_carries_title_description_weight_type_and_edit_url() {
        let p = page(
            "install",
            Kind::HowTo,
            "How to install: \"acme\"",
            "# How to install\n\nBody.\n",
        );
        let out = render_page(
            &p,
            &titles(),
            102,
            Some("https://example.com/edit/main/docs/"),
        )
        .unwrap();
        let (front, body) = split_front(&out);
        let yaml: yaml_serde::Value = yaml_serde::from_str(front).unwrap();
        assert_eq!(yaml["title"].as_str(), Some("How to install: \"acme\""));
        assert_eq!(yaml["description"].as_str(), Some("One line."));
        assert_eq!(yaml["weight"].as_u64(), Some(102));
        assert_eq!(yaml["params"]["diataxis_type"].as_str(), Some("how-to"));
        assert_eq!(
            yaml["editURL"].as_str(),
            Some("https://example.com/edit/main/docs/install.md")
        );
        // Hugo reserves `type` for the layout, so the Diataxis type
        // never sits at the top level.
        assert!(yaml.get("type").is_none());
        assert_eq!(body, "Body.\n");
    }

    #[test]
    fn no_edit_base_writes_no_edit_url() {
        let p = page("install", Kind::HowTo, "How to install", "Body.\n");
        let out = render_page(&p, &titles(), 102, None).unwrap();
        let yaml: yaml_serde::Value = yaml_serde::from_str(split_front(&out).0).unwrap();
        assert!(yaml.get("editURL").is_none());
    }

    #[test]
    fn group_labels_name_the_four_types() {
        let labels: Vec<_> = Kind::ALL.iter().map(|k| group_label(*k)).collect();
        assert_eq!(
            labels,
            ["Tutorials", "How-to guides", "Reference", "Explanation"]
        );
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

        fn repo(&self) -> Repo {
            Repo::open(&self.0).expect("open")
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
    fn export_writes_pages_and_a_separator_per_type_present() {
        let s = Scratch::new("set");
        s.page("zeta-start", &meta("Zeta start", "tutorial"));
        s.page("alpha-start", &meta("Alpha start", "tutorial"));
        s.page("flags", &meta("Flags", "reference"));
        let out = s.0.join("out");
        let mut written = export(&s.repo(), &out, &ExportOptions::default()).unwrap();
        written.sort();
        let names: Vec<_> = written.iter().filter_map(|p| p.file_name()).collect();
        assert_eq!(
            names,
            [
                "alpha-start.md",
                "flags.md",
                "separator-reference.md",
                "separator-tutorial.md",
                "zeta-start.md"
            ]
        );
        let weight = |id: &str| {
            let text = std::fs::read_to_string(out.join(format!("{id}.md"))).unwrap();
            let yaml: yaml_serde::Value = yaml_serde::from_str(split_front(&text).0).unwrap();
            yaml["weight"].as_u64().unwrap()
        };
        // Types read in Diataxis order, the separator heads its group,
        // and pages inside a group sort by title. No weight is zero,
        // because Hugo reads zero as unweighted.
        assert!(weight("separator-tutorial") >= 1);
        assert!(weight("separator-tutorial") < weight("alpha-start"));
        assert!(weight("alpha-start") < weight("zeta-start"));
        assert!(weight("zeta-start") < weight("separator-reference"));
        assert!(weight("separator-reference") < weight("flags"));

        let sep = std::fs::read_to_string(out.join("separator-tutorial.md")).unwrap();
        let yaml: yaml_serde::Value = yaml_serde::from_str(split_front(&sep).0).unwrap();
        assert_eq!(yaml["title"].as_str(), Some("Tutorials"));
        assert_eq!(yaml["sidebar"]["separator"].as_bool(), Some(true));
        assert_eq!(yaml["build"]["render"].as_str(), Some("never"));
        assert_eq!(yaml["build"]["list"].as_str(), Some("always"));
    }

    #[test]
    fn export_reads_both_metadata_forms_and_the_guide_alias() {
        let s = Scratch::new("forms");
        s.page("old", &meta("How to run it", "guide"));
        s.page(
            "new",
            "---\ntitle: New\ndescription: d\ntype: explanation\n---\n\nBody.\n",
        );
        let out = s.0.join("out");
        export(&s.repo(), &out, &ExportOptions::default()).unwrap();
        let old = std::fs::read_to_string(out.join("old.md")).unwrap();
        assert!(old.contains("diataxis_type: how-to"), "{old}");
        assert!(out.join("new.md").exists());
    }

    #[test]
    fn export_fails_on_a_page_that_does_not_parse() {
        // `list` skips a broken page with a note on stderr. A site built
        // that way silently loses the page, so export refuses instead.
        let s = Scratch::new("broken");
        s.page("good", &meta("Good", "tutorial"));
        s.page(
            "bad",
            "<!-- metadata\ntitle: Bad\ntype: overview\n-->\n\nBody.\n",
        );
        let err = export(&s.repo(), &s.0.join("out"), &ExportOptions::default()).unwrap_err();
        assert!(err.to_string().contains("bad"), "{err}");
    }

    #[test]
    fn export_refuses_a_page_whose_name_is_a_separator_file() {
        let s = Scratch::new("collide");
        s.page("separator-tutorial", &meta("Separator", "tutorial"));
        let err = export(&s.repo(), &s.0.join("out"), &ExportOptions::default()).unwrap_err();
        assert!(err.to_string().contains("separator-tutorial"), "{err}");
    }
}

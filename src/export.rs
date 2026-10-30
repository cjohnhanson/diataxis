//! Export a set as content for Hugo.
//!
//! Each page becomes one Markdown file with `---` front matter. A
//! `[[page]]` reference in prose becomes a relative `page.md` link; the
//! check's own scan decides what is a reference, so a set that passes
//! `check` exports. A reference inside code is an example and stays as
//! written. The leading H1 is dropped, because the generator renders
//! the title from the front matter.
//!
//! The sidebar groups pages by Diataxis type. A separator file per
//! type that has pages carries the group label, in the form the Hextra
//! theme's sidebar reads. A type with no pages gets no separator,
//! because Diataxis builds no empty structure.
//!
//! Every file is rendered before any is written, into a directory that
//! is empty or absent. A site built from a partial set, or from a set
//! with stale pages beside it, reads as complete, so either stops the
//! export.

use std::collections::HashMap;

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;

use crate::check::prose_refs;
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

/// The most pages one type can hold. Each type takes a band of a
/// thousand weights: one for its separator and the rest for pages.
const GROUP_LIMIT: usize = 998;

/// Page ids Hugo reads as a section or bundle index, not a page.
const RESERVED_IDS: [&str; 2] = ["_index", "index"];

/// Write every page of the set into `out`, and return the files written.
///
/// `out` must be empty or absent. Fails, writing nothing, on a page that
/// does not parse, a reference that names no page, a reference to
/// another set, and a page whose id is a separator file or a Hugo index.
pub fn export(
    repo: &Repo,
    out: &Utf8Path,
    options: &ExportOptions<'_>,
) -> Result<Vec<Utf8PathBuf>> {
    check_out(out)?;
    if let Some(base) = options.edit_base
        && !(base.starts_with("https://") || base.starts_with("http://"))
    {
        return Err(Error::General(format!(
            "--edit-base {base:?} is not an http(s) URL; give the URL the page files sit under, such as https://github.com/acme/tool/edit/main/docs"
        )));
    }
    let pages = repo.list_strict().map_err(|e| {
        Error::General(format!(
            "{e}; fix the page, and run `diataxis check` to list every fault"
        ))
    })?;
    for page in &pages {
        let separator = Kind::ALL.iter().any(|k| page.id == separator_id(*k));
        if separator || RESERVED_IDS.contains(&page.id.as_str()) {
            return Err(Error::General(format!(
                "page {} has a name export uses for another file; rename the page with a new id",
                page.id
            )));
        }
    }
    let titles: HashMap<&str, &str> = pages.iter().map(|p| (p.id.as_str(), p.title())).collect();

    let mut files: Vec<(Utf8PathBuf, String)> = Vec::new();
    for (index, kind) in Kind::ALL.into_iter().enumerate() {
        let mut group: Vec<&Page> = pages.iter().filter(|p| p.kind() == kind).collect();
        if group.is_empty() {
            continue;
        }
        if group.len() > GROUP_LIMIT {
            return Err(Error::General(format!(
                "{} {kind} pages; export orders at most {GROUP_LIMIT} pages of one type",
                group.len()
            )));
        }
        group.sort_by_key(|p| (p.title().to_lowercase(), p.id.clone()));
        // Starting at 1, because Hugo reads a weight of zero as no
        // weight at all. Both conversions are bounded by the checks above.
        let base = 1000 * u32::try_from(index).expect("four types");
        files.push((
            out.join(format!("{}.md", separator_id(kind))),
            render_separator(kind, base + 1)?,
        ));
        for (position, page) in group.into_iter().enumerate() {
            let weight = base + 2 + u32::try_from(position).expect("bounded by GROUP_LIMIT");
            files.push((
                out.join(format!("{}.md", page.id)),
                render_page(page, &titles, weight, options.edit_base)?,
            ));
        }
    }

    std::fs::create_dir_all(out)
        .map_err(|e| Error::General(format!("cannot create {out}: {e}")))?;
    let mut written = Vec::with_capacity(files.len());
    for (path, text) in files {
        std::fs::write(&path, text)
            .map_err(|e| Error::General(format!("cannot write {path}: {e}")))?;
        written.push(path);
    }
    Ok(written)
}

/// Refuse an output path export cannot own whole.
fn check_out(out: &Utf8Path) -> Result<()> {
    if out.as_str().trim().is_empty() {
        return Err(Error::General(
            "--out takes a directory, not an empty value".to_string(),
        ));
    }
    match std::fs::read_dir(out) {
        Ok(mut entries) => {
            if entries.next().is_some() {
                return Err(Error::General(format!(
                    "{out} is not empty; export writes into an empty or absent directory, so no stale or hand-written file sits among the pages. Remove it or name a new one"
                )));
            }
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) if out.exists() => Err(Error::General(format!(
            "{out} is not a directory ({e}); --out takes a directory"
        ))),
        Err(e) => Err(Error::General(format!("cannot read {out}: {e}"))),
    }
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

/// One page as a front-matter Markdown file, ending in one newline.
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

    // Byte ranges from the check's scan, not a re-serialised document,
    // so every other byte of the page survives.
    let mut out = String::with_capacity(body.len());
    for reference in prose_refs(body) {
        if reference.range.start < cursor {
            continue;
        }
        let target = reference.target.as_str();
        if target.contains(':') {
            return Err(Error::General(format!(
                "page {id}: [[{target}]] names a page in another set, and export resolves references inside one set only; link to the other site's page by its URL"
            )));
        }
        let Some(title) = titles.get(target) else {
            return Err(Error::General(format!(
                "page {id}: [[{target}]] names no page; create the page or correct the reference"
            )));
        };
        out.push_str(&body[cursor..reference.range.start]);
        out.push_str(&format!("[{}]({target}.md)", escape_link_text(title)));
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
    fn brackets_in_a_title_are_escaped_in_the_link_text() {
        let out = rewrite_body("a", "See [[why]].\n", &titles()).unwrap();
        assert_eq!(out, "See [Why acme \\[queues\\]](why.md).\n");
    }

    #[test]
    fn markup_and_a_trailing_backslash_in_a_title_are_escaped() {
        let out = rewrite_body("a", "See [[marked]].\n", &titles()).unwrap();
        assert_eq!(out, "See [Install \\*acme\\* \\\\](marked.md).\n");
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
    fn export_and_check_agree_on_what_is_a_reference() {
        // Emphasis inside the brackets and an escaped bracket are not
        // references to `check`, so export leaves them as written.
        for body in ["See [[*install*]] here.\n", "Escaped \\[[install]] here.\n"] {
            assert!(crate::check::prose_refs(body).is_empty(), "{body}");
            assert_eq!(rewrite_body("a", body, &titles()).unwrap(), body);
        }
    }

    #[test]
    fn a_reference_inside_html_stays_as_written() {
        for body in [
            "A <span title=\"[[install]]\">tip</span> here.\n",
            "<div>\nsee [[absent]]\n</div>\n",
        ] {
            assert!(crate::check::prose_refs(body).is_empty(), "{body}");
            assert_eq!(rewrite_body("a", body, &titles()).unwrap(), body);
        }
    }

    #[test]
    fn an_empty_or_spaced_target_stays_as_written() {
        let body = "Empty [[]] and spaced [[a b]] and [[ ]] stay.\n";
        assert_eq!(rewrite_body("a", body, &titles()).unwrap(), body);
    }

    #[test]
    fn an_empty_description_writes_no_description_key() {
        let mut p = page("install", Kind::HowTo, "How to install", "Body.\n");
        p.frontmatter.description = String::new();
        let out = render_page(&p, &titles(), 102, None).unwrap();
        let yaml: yaml_serde::Value = yaml_serde::from_str(split_front(&out).0).unwrap();
        assert!(yaml.get("description").is_none(), "{out}");
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
            "# How to install\n\nBody.",
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
        // A body with no final newline gets one.
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
        assert!(err.to_string().contains("diataxis check"), "{err}");
    }

    #[test]
    fn export_refuses_a_page_whose_name_is_a_separator_file_or_a_hugo_index() {
        for id in ["separator-tutorial", "_index", "index"] {
            let s = Scratch::new(&format!("collide{id}"));
            s.page(id, &meta("Clash", "tutorial"));
            let err = export(&s.repo(), &s.0.join("out"), &ExportOptions::default()).unwrap_err();
            assert!(err.to_string().contains(id), "{err}");
        }
    }

    #[test]
    fn a_failure_on_a_later_page_writes_nothing() {
        let s = Scratch::new("partial");
        s.page("first", &meta("First", "tutorial"));
        s.page(
            "second",
            "<!-- metadata\ntitle: Second\ntype: reference\n-->\n\nSee [[absent]].\n",
        );
        let out = s.0.join("out");
        export(&s.repo(), &out, &ExportOptions::default()).unwrap_err();
        assert!(!out.exists(), "a failed export left {out}");
    }

    #[test]
    fn export_refuses_a_directory_that_holds_files_and_leaves_it_alone() {
        // The pages directory is the dangerous case: exporting into it
        // would overwrite each source page with its exported form.
        let s = Scratch::new("nonempty");
        s.page("first", &meta("First", "tutorial"));
        let docs = s.0.join("docs");
        let before = std::fs::read_to_string(docs.join("first.md")).unwrap();
        let err = export(&s.repo(), &docs, &ExportOptions::default()).unwrap_err();
        assert!(err.to_string().contains("is not empty"), "{err}");
        assert_eq!(
            std::fs::read_to_string(docs.join("first.md")).unwrap(),
            before
        );
    }

    #[test]
    fn export_writes_into_an_empty_directory() {
        let s = Scratch::new("emptydir");
        s.page("first", &meta("First", "tutorial"));
        let out = s.0.join("out");
        std::fs::create_dir_all(&out).unwrap();
        export(&s.repo(), &out, &ExportOptions::default()).unwrap();
        assert!(out.join("first.md").exists());
    }

    #[test]
    fn an_out_that_is_a_file_or_empty_is_refused_with_its_name() {
        let s = Scratch::new("outfile");
        s.page("first", &meta("First", "tutorial"));
        let file = s.0.join("afile");
        std::fs::write(&file, "x").unwrap();
        let err = export(&s.repo(), &file, &ExportOptions::default()).unwrap_err();
        assert!(err.to_string().contains(file.as_str()), "{err}");
        assert!(err.to_string().contains("not a directory"), "{err}");
        let err = export(&s.repo(), Utf8Path::new(""), &ExportOptions::default()).unwrap_err();
        assert!(err.to_string().contains("empty"), "{err}");
    }

    #[test]
    fn an_edit_base_that_is_not_a_url_is_refused() {
        let s = Scratch::new("editbase");
        s.page("first", &meta("First", "tutorial"));
        for base in ["", "not a url"] {
            let options = ExportOptions {
                edit_base: Some(base),
            };
            let err = export(&s.repo(), &s.0.join("out"), &options).unwrap_err();
            assert!(err.to_string().contains("--edit-base"), "{err}");
        }
    }
}

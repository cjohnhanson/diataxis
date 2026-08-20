//! A documentation set compiled into a binary, and the `docs` subcommand
//! over it.
//!
//! Four sibling tools each carry the same module: a hand-written table
//! of pages, and ten functions to list, find, and search them. The table
//! repeats the title and the description that the page file already
//! states, so the two drift. This module holds the functions once and
//! reads the metadata from the page.
//!
//! A consumer names its page files and nothing else:
//!
//! ```ignore
//! static DOCS: &[(&str, &str)] = diataxis::embed_docs![
//!     "../docs/what-is-acme.md",
//!     "../docs/getting-started.md",
//! ];
//! ```
//!
//! The pages are compiled in, so `acme docs` works from any directory
//! with no files present. That is why the set is embedded rather than
//! read from a store.

use serde::Deserialize;

use crate::error::{Error, Result};
use crate::page::Kind;

/// What a page states about itself, read leniently.
///
/// A page with no type still lists. `diataxis check` is where a missing
/// type is a fault; a reader asking for documentation gets the page.
#[derive(Debug, Deserialize)]
struct DocFrontmatter {
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(rename = "type", default)]
    kind: Option<Kind>,
}

/// One page of bundled documentation.
#[derive(Debug)]
pub struct DocPage {
    /// The file stem, and what a reader types.
    pub slug: String,
    pub title: String,
    pub description: String,
    pub kind: Option<Kind>,
    /// The markdown body, without the frontmatter.
    pub content: String,
}

/// Every bundled page, in the order the consumer listed them.
#[derive(Debug)]
pub struct DocSet {
    pages: Vec<DocPage>,
}

impl DocSet {
    /// Build a set from embedded files, as `(path, content)` pairs.
    ///
    /// [`embed_docs!`](crate::embed_docs) writes those pairs. The slug
    /// is the file stem, so nothing states it twice.
    pub fn from_embedded(files: &[(&str, &str)]) -> Result<Self> {
        let mut pages = Vec::with_capacity(files.len());
        for (path, raw) in files {
            pages.push(parse_page(path, raw)?);
        }
        Ok(Self { pages })
    }

    /// Every page.
    #[must_use]
    pub fn pages(&self) -> &[DocPage] {
        &self.pages
    }

    /// The page an identifier names.
    ///
    /// An exact slug wins, then a slug or title without regard to case,
    /// then a slug prefix that matches one page only. An ambiguous
    /// prefix names nothing, because guessing shows the wrong page.
    #[must_use]
    pub fn find(&self, identifier: &str) -> Option<&DocPage> {
        if let Some(page) = self.pages.iter().find(|p| p.slug == identifier) {
            return Some(page);
        }
        let lower = identifier.to_lowercase();
        if let Some(page) = self.pages.iter().find(|p| p.slug.to_lowercase() == lower) {
            return Some(page);
        }
        if let Some(page) = self.pages.iter().find(|p| p.title.to_lowercase() == lower) {
            return Some(page);
        }
        let prefixed: Vec<&DocPage> = self
            .pages
            .iter()
            .filter(|p| p.slug.starts_with(identifier) || p.slug.starts_with(&lower))
            .collect();
        match prefixed.as_slice() {
            [one] => Some(one),
            _ => None,
        }
    }

    /// Every page whose slug, title, description, or content holds the
    /// query, without regard to case.
    #[must_use]
    pub fn matching(&self, query: &str) -> Vec<&DocPage> {
        let query = query.to_lowercase();
        self.pages
            .iter()
            .filter(|page| {
                page.slug.to_lowercase().contains(&query)
                    || page.title.to_lowercase().contains(&query)
                    || page.description.to_lowercase().contains(&query)
                    || page.content.to_lowercase().contains(&query)
            })
            .collect()
    }

    /// A listing, one line per page.
    ///
    /// The slug column fits the longest slug, so a long page name does
    /// not push one description out of line with the rest.
    #[must_use]
    pub fn format_list(pages: &[&DocPage]) -> String {
        let width = pages
            .iter()
            .map(|p| p.slug.len())
            .max()
            .unwrap_or(0)
            .max(20);
        let mut out = String::new();
        for page in pages {
            out.push_str(&format!("  {:<width$} {}\n", page.slug, page.description));
        }
        out
    }

    /// The whole set as a listing.
    #[must_use]
    pub fn listing(&self) -> String {
        let all: Vec<&DocPage> = self.pages.iter().collect();
        Self::format_list(&all)
    }
}

/// Read one embedded file into a page.
fn parse_page(path: &str, raw: &str) -> Result<DocPage> {
    let slug = path
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .trim_end_matches(".md")
        .to_string();
    let (document, _) = mdstore::parse_any::<DocFrontmatter>(raw)
        .map_err(|e| Error::General(format!("{path}: {e}")))?;
    let title = if document.frontmatter.title.is_empty() {
        slug.clone()
    } else {
        document.frontmatter.title
    };
    Ok(DocPage {
        slug,
        title,
        description: document.frontmatter.description,
        kind: document.frontmatter.kind,
        content: document.body,
    })
}

/// What a caller asks the documentation set for.
///
/// A request, not a print. gaff parses its own arguments and owns its
/// exit codes, so the library returns text and the caller decides what
/// to do with it.
#[derive(Debug, Clone, Copy)]
pub enum Request<'a> {
    /// One line per page.
    List,
    /// One page, by slug, title, or unique prefix.
    Show(&'a str),
    /// A listing of the pages that hold a query.
    Search(&'a str),
    /// Every page, whole, in one stream.
    All,
}

impl DocSet {
    /// Answer a request with the text to print.
    ///
    /// The error names what was asked for, so a caller can print it and
    /// choose its own exit code.
    pub fn render(&self, request: Request<'_>) -> Result<String> {
        match request {
            Request::List => Ok(self.listing()),
            Request::Show(topic) => self
                .find(topic)
                .map(|page| page.content.clone())
                .ok_or_else(|| Error::General(format!("no documentation topic {topic}"))),
            Request::Search(query) => {
                let matches = self.matching(query);
                if matches.is_empty() {
                    return Err(Error::General(format!("no docs matching '{query}'")));
                }
                Ok(Self::format_list(&matches))
            }
            Request::All => Ok(self.everything()),
        }
    }

    /// Every page, whole, in one stream.
    ///
    /// An agent that needs the documentation reads it once instead of
    /// listing and then fetching each page. A comment line opens each
    /// page, because a markdown renderer hides it and a reader
    /// searching the raw text finds it.
    #[must_use]
    pub fn everything(&self) -> String {
        let mut out = String::new();
        for page in &self.pages {
            let kind = page
                .kind
                .map_or_else(|| "untyped".to_string(), |k| k.to_string());
            out.push_str(&format!("<!-- page: {} ({kind}) -->\n\n", page.slug));
            out.push_str(page.content.trim_end());
            out.push_str("\n\n");
        }
        out
    }
}

/// Turn a hand-parsed argument list into a request.
///
/// gaff parses its own arguments, because clap exits 2 on a usage error
/// and 2 is the code gaff reserves for a guard refusing a tool call. A
/// tool in that position calls this and keeps its own exit codes.
///
/// Accepts `docs`, `docs <topic>`, `docs search <query>`, and
/// `docs --all`.
pub fn request_from_args(args: &[String]) -> Result<Request<'_>> {
    match args {
        [] => Ok(Request::List),
        [flag] if flag == "--all" || flag == "-a" => Ok(Request::All),
        [head, rest @ ..] if head == "search" => match rest {
            [query] => Ok(Request::Search(query)),
            [] => Err(Error::General("docs search needs a query".to_string())),
            _ => Err(Error::General("docs search takes one query".to_string())),
        },
        [topic] => Ok(Request::Show(topic)),
        _ => Err(Error::General(
            "docs takes a topic, or `search <query>`, or --all".to_string(),
        )),
    }
}

/// `docs [topic] [query] [--all]`, the subcommand every sibling tool
/// carries.
#[cfg(feature = "cli")]
#[derive(clap::Args)]
pub struct DocsArgs {
    /// Topic slug to show, or "search" to search the docs
    pub topic: Option<String>,

    /// Search query (when topic is "search")
    pub query: Option<String>,

    /// Print every page whole, in one stream
    #[arg(short, long, conflicts_with_all = ["topic", "query"])]
    pub all: bool,
}

#[cfg(feature = "cli")]
impl DocsArgs {
    /// What this invocation asks for.
    #[must_use]
    pub fn request(&self) -> Request<'_> {
        match (self.all, self.topic.as_deref(), self.query.as_deref()) {
            (true, _, _) => Request::All,
            (_, None, _) => Request::List,
            (_, Some("search"), Some(query)) => Request::Search(query),
            (_, Some(topic), _) => Request::Show(topic),
        }
    }
}

#[cfg(feature = "cli")]
impl DocSet {
    /// Run the subcommand and print the result.
    pub fn run(&self, args: &DocsArgs) -> Result<()> {
        if matches!(args.topic.as_deref(), Some("search")) && args.query.is_none() {
            return Err(Error::General("docs search needs a query".to_string()));
        }
        print!("{}", self.render(args.request())?);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    const WHAT: &str = "<!-- metadata\ntitle: \"What is Acme?\"\ndescription: \"Why acme exists\"\ntype: explanation\n-->\n\n# What is Acme?\n\nAcme holds widgets.\n";
    const START: &str = "---\ntitle: \"Getting Started\"\ndescription: \"Run the first job\"\ntype: tutorial\n---\n\nFirst, install it.\n";
    const UNTYPED: &str = "<!-- metadata\ntitle: \"Composed libraries\"\ndescription: \"Declaring other libraries\"\n-->\n\nA library draws on others.\n";

    fn set() -> DocSet {
        DocSet::from_embedded(&[
            ("../docs/what-is-acme.md", WHAT),
            ("../docs/getting-started.md", START),
        ])
        .unwrap()
    }

    #[test]
    fn a_slug_comes_from_the_file_name() {
        assert_eq!(set().pages()[0].slug, "what-is-acme");
    }

    #[test]
    fn the_title_and_type_come_from_the_page() {
        let set = set();
        assert_eq!(set.pages()[0].title, "What is Acme?");
        assert_eq!(set.pages()[0].kind, Some(Kind::Explanation));
        assert_eq!(set.pages()[1].kind, Some(Kind::Tutorial));
    }

    #[test]
    fn both_fence_forms_are_read() {
        let set = set();
        assert!(set.pages()[0].content.starts_with("# What is Acme?"));
        assert_eq!(set.pages()[1].content, "First, install it.");
    }

    #[test]
    fn a_page_with_no_type_still_lists() {
        let set = DocSet::from_embedded(&[("../docs/composition.md", UNTYPED)]).unwrap();
        assert_eq!(set.pages()[0].kind, None);
        assert!(set.listing().contains("composition"));
    }

    #[test]
    fn a_file_that_does_not_parse_names_itself() {
        let broken = "<!-- metadata\ndescription: The workflow: add, sync\n-->\n\nbody\n";
        let err = DocSet::from_embedded(&[("../docs/curation.md", broken)]).unwrap_err();
        assert!(err.to_string().contains("curation.md"), "{err}");
    }

    #[test]
    fn find_takes_an_exact_slug_a_title_or_a_unique_prefix() {
        let set = set();
        assert_eq!(set.find("getting-started").unwrap().slug, "getting-started");
        assert_eq!(set.find("What is Acme?").unwrap().slug, "what-is-acme");
        assert_eq!(set.find("getting").unwrap().slug, "getting-started");
        assert!(set.find("nothing").is_none());
    }

    #[test]
    fn an_ambiguous_prefix_names_nothing() {
        let set =
            DocSet::from_embedded(&[("../docs/how-to-a.md", WHAT), ("../docs/how-to-b.md", WHAT)])
                .unwrap();
        assert!(set.find("how-to").is_none());
    }

    #[test]
    fn search_reads_the_content_as_well_as_the_metadata() {
        let set = set();
        assert_eq!(set.matching("widgets").len(), 1);
        assert_eq!(set.matching("install").len(), 1);
        assert_eq!(set.matching("nothing at all").len(), 0);
    }

    #[test]
    fn all_dumps_every_page_with_a_marker() {
        let text = set().everything();
        assert!(text.contains("<!-- page: what-is-acme (explanation) -->"));
        assert!(text.contains("<!-- page: getting-started (tutorial) -->"));
        assert!(text.contains("Acme holds widgets."));
        assert!(text.contains("First, install it."));
    }

    #[test]
    fn an_untyped_page_says_so_in_the_dump() {
        let set = DocSet::from_embedded(&[("../docs/composition.md", UNTYPED)]).unwrap();
        assert!(
            set.everything()
                .contains("<!-- page: composition (untyped) -->")
        );
    }

    #[test]
    fn render_answers_each_request() {
        let set = set();
        assert!(set.render(Request::List).unwrap().contains("what-is-acme"));
        assert!(
            set.render(Request::Show("what-is-acme"))
                .unwrap()
                .contains("Acme holds widgets.")
        );
        assert!(
            set.render(Request::Search("widgets"))
                .unwrap()
                .contains("what-is-acme")
        );
        assert!(set.render(Request::All).unwrap().contains("<!-- page:"));
    }

    #[test]
    fn render_names_what_it_could_not_find() {
        let set = set();
        let err = set.render(Request::Show("absent")).unwrap_err().to_string();
        assert!(err.contains("absent"), "{err}");
        let err = set.render(Request::Search("zzz")).unwrap_err().to_string();
        assert!(err.contains("zzz"), "{err}");
    }

    #[test]
    fn a_hand_parsed_argument_list_becomes_a_request() {
        let args =
            |parts: &[&str]| -> Vec<String> { parts.iter().map(|s| (*s).to_string()).collect() };
        assert!(matches!(
            request_from_args(&args(&[])).unwrap(),
            Request::List
        ));
        assert!(matches!(
            request_from_args(&args(&["--all"])).unwrap(),
            Request::All
        ));
        assert!(matches!(
            request_from_args(&args(&["getting-started"])).unwrap(),
            Request::Show("getting-started")
        ));
        assert!(matches!(
            request_from_args(&args(&["search", "widgets"])).unwrap(),
            Request::Search("widgets")
        ));
        assert!(request_from_args(&args(&["search"])).is_err());
    }
}

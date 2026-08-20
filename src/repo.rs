//! Reading and writing pages in one documentation set.

use camino::{Utf8Path, Utf8PathBuf};

use crate::config::DiataxisConfig;
use crate::error::{Error, Result};
use crate::page::{Kind, Page, PageFrontmatter};

/// One documentation set, rooted at the directory holding diataxis.yml.
pub struct Repo {
    root: Utf8PathBuf,
    config: DiataxisConfig,
}

/// What `page create` is given beyond the title.
#[derive(Default)]
pub struct CreatePageOptions {
    pub description: Option<String>,
    pub tags: Option<String>,
    pub body: Option<String>,
    /// The file stem, when the slug from the title is not wanted.
    pub id: Option<String>,
}

/// What `page edit` may change. A field left `None` is left alone.
#[derive(Default)]
pub struct EditPageOptions<'a> {
    pub title: Option<&'a str>,
    pub description: Option<&'a str>,
    pub kind: Option<Kind>,
    pub tags: Option<&'a str>,
    pub body: Option<&'a str>,
}

/// One matching line, from `search`.
pub struct SearchResult {
    pub id: String,
    pub kind: Kind,
    pub line_number: usize,
    pub line: String,
}

impl Repo {
    /// Open the set rooted here. The config file must exist.
    pub fn open(root: &Utf8Path) -> Result<Self> {
        let marker = root.join("diataxis.yml");
        let text =
            std::fs::read_to_string(&marker).map_err(|_| Error::NoStore(root.to_string()))?;
        let config: DiataxisConfig = yaml_serde::from_str(&text)?;
        Ok(Self {
            root: root.to_owned(),
            config,
        })
    }

    /// Write diataxis.yml and make the pages directory.
    ///
    /// An existing directory of pages is adopted rather than replaced,
    /// so a repository with docs already written runs `init` once and
    /// keeps every page.
    pub fn init(root: &Utf8Path) -> Result<()> {
        let marker = root.join("diataxis.yml");
        if marker.exists() {
            return Err(Error::General(format!("{marker} already exists")));
        }
        let config = DiataxisConfig::default();
        std::fs::write(&marker, yaml_serde::to_string(&config)?)?;
        let dir = root.join(&config.docs_dir);
        std::fs::create_dir_all(&dir)?;
        println!("diataxis: wrote {marker}");
        println!("diataxis: pages live in {dir}");
        Ok(())
    }

    /// The directory holding the pages.
    pub fn docs_dir(&self) -> Result<Utf8PathBuf> {
        let dir = mdstore::store::document_dir(self.root.as_std_path(), &self.config.docs_dir)?;
        Utf8PathBuf::from_path_buf(dir)
            .map_err(|p| Error::General(format!("non-UTF-8 docs directory {}", p.display())))
    }

    #[must_use]
    pub fn title(&self) -> String {
        if self.config.title.is_empty() {
            self.root.file_name().unwrap_or("documentation").to_string()
        } else {
            self.config.title.clone()
        }
    }

    /// The file a page id names, whether or not it exists.
    pub fn path_of(&self, id: &str) -> Result<Utf8PathBuf> {
        if !mdstore::store::is_plain_stem(id) {
            return Err(Error::General(format!("{id} is not a page name")));
        }
        Ok(self.docs_dir()?.join(format!("{id}.md")))
    }

    /// Every page in this set, in id order.
    ///
    /// A file that does not parse is skipped and named on stderr. One
    /// broken page never hides the rest.
    pub fn list(&self) -> Result<Vec<Page>> {
        let dir = self.docs_dir()?;
        let mut pages = Vec::new();
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(pages),
            Err(e) => return Err(e.into()),
        };
        let mut paths: Vec<Utf8PathBuf> = Vec::new();
        for entry in entries {
            let path = Utf8PathBuf::from_path_buf(entry?.path())
                .map_err(|p| Error::General(format!("non-UTF-8 page {}", p.display())))?;
            if path.extension() == Some("md") {
                paths.push(path);
            }
        }
        paths.sort();
        for path in paths {
            let Some(stem) = path.file_stem() else {
                continue;
            };
            match read_page(&path, stem) {
                Ok(page) => pages.push(page),
                Err(e) => eprintln!("diataxis: skipping {path}: {e}"),
            }
        }
        Ok(pages)
    }

    /// The page this id or prefix names.
    pub fn get(&self, selector: &str) -> Result<Page> {
        let pages = self.list()?;
        crate::selector::pick(pages, selector)
    }

    /// Write a new page, and return it.
    pub fn create(&self, title: &str, kind: Kind, options: CreatePageOptions) -> Result<Page> {
        let id = match options.id {
            Some(id) => id,
            None => mdstore::slug::slugify(title),
        };
        let path = self.path_of(&id)?;
        if path.exists() {
            return Err(Error::PageExists(id));
        }
        let page = Page {
            id,
            frontmatter: PageFrontmatter {
                title: title.to_string(),
                description: options.description.unwrap_or_default(),
                kind,
                tags: split_list(options.tags.as_deref()),
            },
            body: options.body.unwrap_or_default(),
            // A new page is written in the comment form. Every existing
            // page in the sibling repositories uses it, and a forge
            // renders a leading `---` as a rule plus the raw keys.
            fence: mdstore::Fence::Comment,
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, render(&page)?)?;
        Ok(page)
    }

    /// Change a page in place, keeping the fence it was written in.
    pub fn edit(&self, selector: &str, options: EditPageOptions<'_>) -> Result<Page> {
        let mut page = self.get(selector)?;
        if let Some(title) = options.title {
            page.frontmatter.title = title.to_string();
        }
        if let Some(description) = options.description {
            page.frontmatter.description = description.to_string();
        }
        if let Some(kind) = options.kind {
            page.frontmatter.kind = kind;
        }
        if let Some(tags) = options.tags {
            page.frontmatter.tags = split_list(Some(tags));
        }
        if let Some(body) = options.body {
            page.body = body.to_string();
        }
        std::fs::write(self.path_of(&page.id)?, render(&page)?)?;
        Ok(page)
    }

    /// Remove a page.
    pub fn delete(&self, selector: &str) -> Result<Page> {
        let page = self.get(selector)?;
        std::fs::remove_file(self.path_of(&page.id)?)?;
        Ok(page)
    }

    /// Every line matching the pattern, across every page.
    pub fn search(&self, pattern: &str) -> Result<Vec<SearchResult>> {
        let regex = regex::RegexBuilder::new(pattern)
            .case_insensitive(true)
            .build()?;
        let mut hits = Vec::new();
        for page in self.list()? {
            for (index, line) in page.body.lines().enumerate() {
                if regex.is_match(line) {
                    hits.push(SearchResult {
                        id: page.id.clone(),
                        kind: page.kind(),
                        line_number: index + 1,
                        line: line.trim().to_string(),
                    });
                }
            }
        }
        Ok(hits)
    }
}

/// Read one page file.
fn read_page(path: &Utf8Path, stem: &str) -> Result<Page> {
    let text = std::fs::read_to_string(path)?;
    let (document, fence) = mdstore::parse_any::<PageFrontmatter>(&text)?;
    Ok(Page {
        id: stem.to_string(),
        frontmatter: document.frontmatter,
        body: document.body,
        fence,
    })
}

/// Serialize a page back to the form it came in.
fn render(page: &Page) -> Result<String> {
    let document = mdstore::Document {
        frontmatter: page.frontmatter.clone(),
        body: page.body.clone(),
    };
    Ok(mdstore::serialize_with(&document, page.fence)?)
}

/// Split a comma-separated option into trimmed, non-empty parts.
fn split_list(raw: Option<&str>) -> Vec<String> {
    raw.map(|s| {
        s.split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect()
    })
    .unwrap_or_default()
}

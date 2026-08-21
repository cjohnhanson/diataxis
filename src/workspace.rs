//! The vantage a command runs from: this set plus the sets it declares.
//!
//! Every command runs *from* one documentation set. What it can see is
//! that set's dependency closure, so a repository's own pages never see
//! a private set that annotates them. The repository does not declare
//! it, and it cannot: the target does not exist for the other people
//! who clone the repository.

use camino::Utf8Path;
use mdstore::registry::{Registry, RegistryLocator};
use mdstore::snapshot::{DocId, DocumentSource, Entry, Snapshot};
use mdstore::store::{FetchingLocator, LocalPaths, StoreContent, StoreGraph, StoreRef};

use crate::config::DiataxisConfig;
use crate::error::{Error, Result};
use crate::page::{Page, PageFrontmatter};

/// Reads pages out of a store directory.
pub struct PageSource;

impl DocumentSource for PageSource {
    type Doc = Page;

    fn load(
        &self,
        content: &StoreContent,
        skipped: &mut Vec<String>,
    ) -> mdstore::Result<Vec<Entry<Page>>> {
        // A dependency's own config chooses its pages directory, so the
        // value is guarded: it may not be absolute or climb out of the
        // store.
        let dir_name = if content.exists("diataxis.yml") {
            let text = content.read("diataxis.yml")?;
            yaml_serde::from_str::<DiataxisConfig>(&text)
                .map(|c| c.docs_dir)
                .unwrap_or_else(|_| "docs".to_string())
        } else {
            "docs".to_string()
        };
        if let Some(dir) = content.dir() {
            mdstore::store::document_dir(dir, &dir_name)?;
        }

        let scan = content.scan(&dir_name)?;
        for (path, why) in scan.skipped {
            skipped.push(format!("{} ({why})", path.display()));
        }

        let mut pages = Vec::new();
        for entry in scan.entries {
            // One unreadable page never takes down a store, and neither
            // does one that declares no type. A set is read to answer a
            // question, and one bad file must not empty the answer.
            let text = match content.read(&entry.path.to_string_lossy()) {
                Ok(text) => text,
                Err(e) => {
                    skipped.push(format!("{} ({e})", entry.path.display()));
                    continue;
                }
            };
            match mdstore::parse_any::<PageFrontmatter>(&text) {
                Ok((document, fence)) => pages.push(Entry {
                    id: entry.stem.clone(),
                    doc: Page {
                        id: entry.stem,
                        frontmatter: document.frontmatter,
                        body: document.body,
                        fence,
                    },
                }),
                Err(e) => skipped.push(format!("{} ({e})", entry.path.display())),
            }
        }
        Ok(pages)
    }

    /// Everything a page points at: `[[ref]]` references in its body,
    /// parsed against the aliases the holding store declares.
    fn references(&self, doc: &Page, member: usize, graph: &StoreGraph) -> Vec<StoreRef> {
        let aliases = graph.config(member).aliases();
        let mut refs = Vec::new();
        for text in crate::check::body_refs(&doc.body) {
            let parsed = StoreRef::parse(&text, &aliases);
            if !refs.contains(&parsed) {
                refs.push(parsed);
            }
        }
        refs
    }

    /// Resolve a reference inside one set.
    ///
    /// A page id is its file stem and a reader types it whole, so an
    /// exact match is the only match. A prefix is convenient at a
    /// prompt and wrong in a link: a page added later can capture a
    /// reference that already resolved.
    fn resolve_local(&self, id: &str, entries: &[Entry<Page>]) -> Option<usize> {
        entries.iter().position(|entry| entry.id == id)
    }
}

/// One row of `store list`.
pub struct StoreRow {
    /// Empty for the vantage set itself.
    pub alias: String,
    pub source: String,
    pub pages: usize,
    pub unavailable: Option<String>,
    /// True when a cache holds this set.
    pub remote: bool,
    /// The time since the last fetch, for a remote set.
    pub age: Option<String>,
}

/// A loaded vantage.
pub struct Workspace {
    snapshot: Snapshot<PageSource>,
}

impl Workspace {
    /// Load the vantage set and its closure from what is on this
    /// machine. No command that reads reaches the network.
    pub fn open(root: &Utf8Path) -> Result<Self> {
        Self::open_with(root, false)
    }

    /// Load the closure, and clone a remote set that is absent. Only
    /// `store sync` uses this.
    pub fn open_fetching(root: &Utf8Path) -> Result<Self> {
        Self::open_with(root, true)
    }

    fn open_with(root: &Utf8Path, fetching: bool) -> Result<Self> {
        let registry = Registry::load().unwrap_or_default();
        let graph = if fetching {
            let locator = RegistryLocator::new(registry, FetchingLocator);
            StoreGraph::open(root.as_std_path(), &locator)?
        } else {
            let locator = RegistryLocator::new(registry, LocalPaths);
            StoreGraph::open(root.as_std_path(), &locator)?
        };
        Ok(Workspace {
            snapshot: Snapshot::load(graph, &PageSource)?,
        })
    }

    /// One row per closure member, for `store list`.
    pub fn store_members(&self) -> Vec<StoreRow> {
        self.snapshot
            .graph
            .members
            .iter()
            .enumerate()
            .map(|(i, m)| StoreRow {
                alias: m.alias_path.join("/"),
                remote: m.content.as_ref().is_some_and(StoreContent::is_remote),
                age: m.content.as_ref().and_then(StoreContent::fetch_age),
                source: if m.alias_path.is_empty() {
                    // The vantage set: show where it is, not the "."
                    // it was opened with.
                    m.content
                        .as_ref()
                        .and_then(StoreContent::dir)
                        .map(|p| {
                            p.canonicalize()
                                .unwrap_or_else(|_| p.to_path_buf())
                                .display()
                                .to_string()
                        })
                        .unwrap_or_default()
                } else {
                    match &m.source {
                        mdstore::StoreSource::Path(p) => p.display().to_string(),
                        mdstore::StoreSource::Git { url, rev } => match rev {
                            Some(rev) => format!("{url}@{rev}"),
                            None => url.clone(),
                        },
                        mdstore::StoreSource::Blob { url } => url.clone(),
                    }
                },
                pages: self.snapshot.member_documents(i).count(),
                unavailable: m.unavailable.clone(),
            })
            .collect()
    }

    /// Fetch every declared remote set into the cache.
    pub fn sync_all(&self) -> Vec<(String, Result<()>)> {
        let mut results = Vec::new();
        for (i, member) in self.snapshot.graph.members.iter().enumerate() {
            if i == 0 || matches!(member.source, mdstore::StoreSource::Path(_)) {
                continue;
            }
            let alias = member.alias_path.join("/");
            let outcome = mdstore::store::sync_source(&member.source).map_err(Error::Store);
            results.push((alias, outcome));
        }
        results
    }

    /// Every reference in the closure that names no page.
    ///
    /// A reference into a declared set resolves here, where `check`
    /// reading one set on its own has to skip it.
    pub fn missing(&self) -> Vec<String> {
        self.snapshot.missing()
    }

    /// Pages of the vantage set only, in id order.
    pub fn local(&self) -> Vec<&Page> {
        self.snapshot
            .member_documents(0)
            .map(|(_, entry)| &entry.doc)
            .collect()
    }

    /// The qualified id of a page, for a message a reader sees.
    pub fn qualify(&self, id: DocId) -> String {
        self.snapshot.qualify(id)
    }
}

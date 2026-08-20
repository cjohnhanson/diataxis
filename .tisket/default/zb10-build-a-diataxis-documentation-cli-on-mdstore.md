---
title: Build a Diataxis documentation CLI on mdstore
status: todo
priority: null
assignee: null
due_date: null
labels: []
depends_on: []
created: 2026-08-18T21:15:41Z
updated: 2026-08-18T21:15:41Z
---

Five repositories ship a `docs/` directory with no shared shape and no check on it. Each page already declares a Diátaxis type in an HTML metadata comment, and nothing reads that field except the bundled-docs listing. A page can claim `type: guide` and hold a reference table, and no command says so.

Build a documentation CLI on mdstore, with the ergonomics of tisket, zettel, almanac, and missouri. It manages a documentation set as a typed store: one page, one Diátaxis type. It creates pages of a declared type, lists them by type, prints the four-quadrant map of a set, checks type discipline and link health, and serves the set over MCP.

The five sibling repositories are the first consumers.

## Out of scope

Rendering, publishing, and static-site generation. Sphinx and Read the Docs already do that. The tool governs the source files.

## Scratch Notes

# Draft plan, first pass

Status: draft for discussion. Five decisions below are open. Nothing is built.

## The shape it inherits

The four sibling tools share one skeleton. This tool copies it rather than inventing anything:

- `<name>.yml` at the store root, holding a `docs_dir` key. Found upward from the cwd.
- Global `--root` (literal directory, no walk) and `--home` (the configured root store).
- `stores.yml` composition under local aliases. Nearer store wins a name collision. Declared stores stay read-only.
- An `intent()` function classifying every leaf command as none, read, or write. Exhaustive, so a new subcommand does not compile until it is classified.
- Commands every sibling carries: `init`, `prime`, `docs [topic]`, `check`, `search`, `store list|sync|root`, `serve`, plus hidden `gen-man` and `gen-completions`.
- Bundled documentation compiled in with `include_str!`, listed in `docs.rs`.
- Module layout: `cli.rs`, `config.rs`, `docs.rs`, `error.rs`, `repo.rs`, `workspace.rs`, `serve.rs`, `selector.rs`, `mangen.rs`, `main.rs`, `lib.rs`.
- `mdstore::snapshot::Snapshot` for the link graph. It already gives forward links, backlinks, orphans, neighbourhood, and cross-store resolution.
- Release profile: `strip = "symbols"`, thin LTO, one codegen unit.
- A missouri suite in `tests/missouri`, since tisket already proves that pattern.

## What is specific to this tool

A page is a markdown document whose frontmatter declares one Diátaxis type: `tutorial`, `how-to`, `reference`, or `explanation`. The type is the whole point. Every command reads it.

### Command sketch

```
<name> init
<name> page create "Title" --type tutorial [--description ...] [--body ...]
<name> page list [--type reference] [--untyped]
<name> page show <id>
<name> page edit <id> [--type explanation] [--title ...] [--description ...]
<name> page delete <id>
<name> map                       # the four quadrants, with the pages in each
<name> compass [<id>]            # the two questions and the decision table
<name> check                     # type discipline, link health, coverage
<name> search <pattern>
<name> store list | sync | root
<name> serve
<name> docs [topic]
<name> prime
```

`map` is the flagship read. It prints the set as the two-by-two grid, not as a list, so a gap is visible: a product with no explanation, or eleven how-to guides and one tutorial.

`compass` prints the two questions and the table that turns a pair of answers into a type. Given a page id, it prints the page's declared type alongside. It does not classify the page. Classification needs a reader, and a tool that guessed would be inventing a fact.

### What `check` can prove mechanically

- Every page declares a type, and the type is one of the four.
- No page declares two types.
- Every internal link resolves.
- Orphan pages.
- Coverage: each of the four types is present. Advisory, not an error.
- A how-to title states a goal. The framework's own rule: "How to integrate application performance monitoring", not "Integrating application performance monitoring". A title check is mechanical.

### What `check` cannot prove

Whether prose in a reference page is really explanation. That is a judgement, and a heuristic that guessed would produce confident noise. Decision 4 covers whether to attempt advisory signals at all.

## Decisions

1. **Name.** Settled: `diataxis`. Repository, crate, and binary. The name is free on crates.io.

2. **Where pages live.** Documentation is the published artefact, unlike an issue or a note. `docs_dir: docs` points the store at the existing `docs/` directory of each repository. The alternative, a hidden `.taxis/`, matches the siblings but splits the docs from where every reader and every static-site build expects them. Default: `docs`.

3. **The frontmatter form.** This is the crux. Every existing page carries the metadata in an HTML comment:

   ```
   <!-- metadata
   title: "What is Tisket?"
   type: explanation
   -->
   ```

   mdstore parses `---` fences and nothing else. Two ways out:

   a. Convert every page to YAML frontmatter. Cheapest for the new tool, and mdstore stays as it is. GitHub renders a leading `---` as a horizontal rule followed by the raw keys, so every docs page on GitHub grows a header of visible metadata. The comment form exists to avoid exactly that.

   b. Teach mdstore a second delimiter, the comment form, alongside `---`. The right layer, since mdstore is the storage library and this is a storage concern. It costs a change to a crate three tools already depend on, plus a decision about which form `serialize` writes.

   Default: (b), on the reading that the comment form was a deliberate choice and worth keeping.

4. **Advisory type-discipline signals.** Should `check` flag likely leaks, such as a wide table inside a tutorial or a "why" section inside reference? Useful, and inherently approximate. Default: no in version one. Ship what is provable, add signals later behind a flag if the provable set proves too thin.

5. **MCP surface.** Every sibling serves. A documentation set is the thing an agent most wants to read, so `serve` earns its place. The question is write access: tisket allows appending to scratch and nothing else. Default: read-only in version one.


## The web view, and what mdbook is today

Correction to the premise this plan started from: mdstore holds no mdbook code. It holds one issue, `32l8` in the mdstore tracker, status todo, with seven open decisions and two named risks. `grep mdbook src/ Cargo.toml` returns nothing, and the feature list holds `mcp` alone.

What the issue does establish, from a probe that ran: mdbook renders a `Book` built entirely in memory, with synthetic chapters carrying `source_path: None`, no `SUMMARY.md`, no `book.toml`, and no markdown on disk. The one disk requirement is an existing `root/src` directory, which may be empty. That gives a store a read-only HTML site with client-side search, a dark mode, and a mobile layout, for no server and no write path.

The issue names four consumers: almanac, tisket, zettel, "and a fourth on the way". This tool is the fourth.

Two consequences for this plan:

- A `serve` or `render` command here does not build its own HTML. It waits for the mdstore feature and supplies the two tool-specific pieces the issue asks each consumer for: a spine, and titles. The spine for a documentation set is the obvious one, and it is the reason this tool exists: the four types, in the order tutorial, how-to, reference, explanation.
- Decision 3 below and decision 4 of `32l8`, whether frontmatter renders as a table or stays hidden, are the same question seen from two sides. Settle them together.

## Order of work

1. Decide 1 to 3. They fix the crate name, the store layout, and whether mdstore changes.
2. If decision 3 goes to (b), change mdstore first and release it.
3. Scaffold the crate from tisket's layout. Config, error, page type, repo, workspace.
4. `init`, `page create`, `page list`, `page show`. The store works end to end.
5. `check`, then `map`, then `compass`.
6. `search`, `store`, `docs`, `prime`, `gen-man`, `gen-completions`.
7. `serve`.
8. The missouri suite, then the docs, then adoption by the five repositories.
## Built, staged, not committed

Name settled: diataxis. Crate, binary, repository.

Correction to an earlier note in this scratch: mdstore does hold the book feature. It sits on branch feat/store-as-a-book, commit be9a5de, as src/book.rs behind a `book` feature. The earlier note read the checked-out branch only. The module gives a Chaptered trait, to_book, render_html, [[id]] rewriting, grouping by part title, and sub-pages. A consumer supplies titles and a group. For this tool the group is the type, which means one open question: to_book orders groups through a BTreeMap, so the sidebar would read explanation, how-to, reference, tutorial. Diataxis order is tutorial, how-to, reference, explanation.

mdstore work, branch feat/comment-fence, staged: Fence, parse_any, parse_with, serialize_with, split_with. Tracked as mdstore issue 1brb. Gates green.

Crate as it stands: init, page create/list/show/edit/delete/path, map, compass, check, search, docs, prime, gen-man, gen-completions. Root resolution through mdstore::resolve, with --root, --home, and the announce line. Fourteen tests pass, clippy and fmt clean.

Proven against real data. Four sibling doc sets copied to a scratch directory with a diataxis.yml and read:
- tisket: workflow declares guide, and its title states no goal.
- missouri: writing-tests, same fault.
- almanac: composition.md declares no type; curation.md holds an unquoted colon in its description, so its metadata is not valid YAML. Neither file parses.
- zettel: clean, one advice for no how-to pages.

The zettel run first reported [[note-id]] as a broken link. That was the tool's fault, not the page's: the reference sits inside backticks. check now strips fenced blocks and code spans before it scans, with four tests.

Not built: store list/sync/root, serve, the book render command, the missouri suite, README, the remaining two doc pages, CI, flake.

Decisions 2 to 5 in the plan stand at their defaults.
## The docs subcommand as a library

Verified: the recollection about gaff is right. gaff/src/cli.rs states it at the top. gaff parses arguments by hand because clap exits 2 on a usage error, and 2 is the code gaff reserves for a guard refusing a tool call. No gaff fault may block a session. gaff has no clap dependency at all. kersh has none either, and no docs command, though it holds docs/kersh.md.

So the library splits in two, and the split is the point.

Core, no clap: DocSet::from_embedded(&[(path, content)]), find, matching, listing, everything, and render(Request) returning a String. Request is List, Show, Search, or All. A caller prints the text and picks its own exit code. request_from_args maps a hand-parsed argument list to a Request, so gaff adopts in two lines.

Behind the default cli feature: DocsArgs, which derives clap::Args, and DocSet::run, which prints what render returns. clap, clap_complete, clap_mangen, colored, miette, and sigpipe are all optional and all sit behind that feature. The binary declares required-features = ["cli"]. cargo build --no-default-features --lib compiles with none of them.

The duplication this removes: four repositories each carry a docs.rs of 130 to 310 lines, holding the same ten functions and a hand-written table that repeats the title and the description each page already states. embed_docs! takes paths only:

    static DOCS: &[(&str, &str)] = diataxis::embed_docs![
        "../docs/what-is-acme.md",
        "../docs/getting-started.md",
    ];

The slug is the file stem. The title, the description, and the type come from the page. Nothing states them twice.

docs --all dumps every page whole, for an agent that wants one read instead of a list and then a fetch. Each page opens with a comment line: <!-- page: what-is-diataxis (explanation) -->. A markdown renderer hides it, a reader searching raw text finds it. Pages are read leniently, so a page with no type still lists and reports as untyped; almanac has one.

Thirty tests pass across both feature sets, including an integration test that drives the library the way gaff would, with no clap in the build. clippy and fmt clean on --all-features and on --no-default-features.
## No list at all

The list is gone. A build script embeds the whole set.

src/build.rs carries no dependencies. embed_docs() reads docs_dir from diataxis.yml beside Cargo.toml, defaults to docs, and refuses a value that is absolute or climbs out. It writes an array of (path, include_str!(path)) pairs into OUT_DIR, and prints rerun-if-changed for the directory and for every page. The directory line is what catches a page added or removed; the per-file lines catch an edit.

A consumer writes two lines. In build.rs: diataxis::build::embed_docs().unwrap(). In the crate: static DOCS: &[(&str, &str)] = diataxis::embedded_docs!();

embed_docs![paths...] stays for a consumer that wants a subset or a fixed order.

Feature tree: build carries nothing, store carries camino, mdstore, regex, serde, thiserror, yaml_serde, and cli adds store plus clap and the rest. default = ["cli"]. cargo tree --no-default-features --features build prints diataxis and no dependency at all, so a build-dependency on this crate costs one compile unit.

This crate embeds its own set the same way. build.rs points a #[path] module at src/build.rs, because a crate cannot depend on itself.

Proven: diataxis page create wrote a third page, cargo build picked it up with no source edited, and diataxis docs listed it.

The third page is the adoption how-to. It states the manifest entries, the two lines, both wiring paths for a clap tool and a hand-parsing tool, and the eight functions to delete from the old module.

32 unit tests, 3 integration tests, 5 build tests. clippy and fmt clean on --all-features, on build alone, and on store alone.

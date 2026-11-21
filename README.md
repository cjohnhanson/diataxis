# diataxis

diataxis manages a documentation set as typed markdown. Every page
declares one of four types in its frontmatter: tutorial, how-to,
reference, or explanation. Every command reads that field. `map` shows
the set as the four quadrants, so a gap is visible. `check` reports what
the files prove, and nothing else.

It is also a library. Five sibling tools each carried the same `docs`
subcommand, with a hand-written table of pages that repeated the title
and description each page already states. diataxis holds that once.

## Install

```sh
cargo install diataxis
```

From a checkout instead:

```sh
cargo install --git https://github.com/cjohnhanson/diataxis
```

Two things the build needs. Rust 1.88 or later: the code uses
let-chains, and a locked dependency asks for 1.88 itself. And a C
compiler, because the store reads a remote over HTTPS and that TLS stack
builds a C library. On Debian and Ubuntu that is `build-essential`; on
macOS, the Xcode command line tools.

This is a library first. A consumer that only reads and writes pages
takes the `store` feature and needs neither the command nor its
dependencies:

```toml
diataxis = { version = "0.2", default-features = false, features = ["store"] }
```

## Usage

`diataxis init` writes `diataxis.yml` and makes `docs/`. `page create`
writes a page of one type, and `page list` prints the pages with their
types:

```console
$ diataxis init
diataxis: wrote [CWD]/diataxis.yml
diataxis: pages live in [CWD]/docs

$ diataxis page create "How to rotate a key" -T how-to --body "Run acme rotate, then restart."
how-to-rotate-a-key (how-to)

$ diataxis page create "Getting started" -T tutorial
getting-started (tutorial)

$ diataxis page list
getting-started              tutorial     Getting started
how-to-rotate-a-key          how-to       How to rotate a key

```

`check` reports type faults and broken links. A type with no pages is
advice, not a fault. `search` prints every line that matches a pattern:

```console
$ diataxis check
advice  the set                      no reference pages
advice  the set                      no explanation pages

$ diataxis search rotate
how-to-rotate-a-key:1  how-to  Run acme rotate, then restart.

```

`map` shows the four quadrants and the pages in each. `export --out
<DIR>` writes the pages as front-matter Markdown for a site generator.
`store list`, `store sync`, and `store root` manage the sets this set
reads. `docs` prints the bundled documentation, and
[the CLI reference](docs/cli-reference.md) lists every command.

## The four types

Diátaxis identifies four reader needs and one form for each. Two
questions separate them. Does a page inform **action** or **cognition**?
Does it serve a reader **acquiring** skill, or one **applying** it.

| | acquisition | application |
|---|---|---|
| **action** | tutorial | how-to guide |
| **cognition** | explanation | reference |

`diataxis compass` prints the two questions and the four types:

```console
$ diataxis compass
Two questions name a page's type.

  1. Does it inform action or cognition?
  2. Does it serve acquisition of skill, or application?

  tutorial     action     acquisition  a lesson: the reader learns by doing, and the writer owns the outcome
  how-to       action     application  directions: the reader is competent and already knows the goal
  reference    cognition  application  neutral description of the machinery, consulted during work
  explanation  cognition  acquisition  a discursive treatment of one topic, read away from the work

```

It does not classify a page. Classification needs a reader, and a tool
that guessed would be inventing a fact.

## How it works

`diataxis.yml` governs one documentation set. It names the pages
directory, `docs` by default, and the title `map` prints. Without a
title, `map` prints the name of the set's directory. Documentation is
the published artefact, unlike an issue or a note, so it stays where
readers and site builds look for it.

```yaml {filename="diataxis.yml"}
docs_dir: docs
title: acme
```

A page is a markdown document whose frontmatter declares a type:

```markdown {filename="docs/how-to-rotate-the-signing-key.md"}
<!-- metadata
title: "How to rotate the signing key"
description: "Replace a key without dropping a session"
type: how-to
-->

# How to rotate the signing key
```

```console
$ diataxis page show how-to-rotate-the
how-to-rotate-the-signing-key

  Title:        How to rotate the signing key
  Type:         how-to
  Description:  Replace a key without dropping a session

# How to rotate the signing key

```

Frontmatter comes in two forms. The `---` form is what most markdown
tools expect. The comment form hides the same keys from every markdown
renderer, so a page read on a git forge shows no block of raw metadata.
diataxis reads both and writes the comment form.

A page id is its file stem. There is no random prefix, because the id
becomes a published path and a reader bookmarks it.

## What check proves

A check is worth reading only if every finding is true. diataxis
reports what the files prove: a markdown file that is not a page, a
how-to title that states no goal, a `[[reference]]` or a relative link
that resolves to nothing. A type with no pages comes back as advice
rather than a fault.

It does not judge prose. Whether an explanation has crept into a
reference page needs a reader. A reference inside a code span or a
fenced block is not a link, so a page that quotes syntax is not
reported.

## Composed sets

A set can draw on other sets. Declare them in `stores.yml`, each under a
local alias:

```yaml {filename="stores.yml"}
format: 2
stores:
  - alias: shared
    path: ../team-docs
```

A page then writes `[[shared:onboarding]]`. `store list` shows every set
in the closure with its page count:

```console
$ diataxis store list
(this set)  [CWD]  3 page(s)
shared  ../team-docs  1 page(s)

```

`check` does not resolve a qualified reference. mdstore exposes no
per-reference resolution across a closure, so `check` reads one set and
reports a set that did not load, which is why a reference into it cannot
be answered.

## The docs subcommand, as a library

A tool that ships its own documentation takes diataxis as a
dependency. Nothing lists the pages:

```toml
[build-dependencies]
diataxis = { version = "0.2", default-features = false, features = ["build"] }

[dependencies]
diataxis = { version = "0.2", default-features = false, features = ["cli"] }
```

```rust,no_run
// build.rs
fn main() -> std::io::Result<()> {
    diataxis::build::embed_docs()
}
```

```rust
static DOCS: &[(&str, &str)] = diataxis::embedded_docs!();
```

The build script reads `docs_dir` from `diataxis.yml`, embeds every page
it holds, and tells cargo to rebuild when one changes. A page added to
the directory is in the next build. The title, the description, and the
type come from the page, so no line of Rust repeats them.

Three features serve three kinds of consumer. `build` carries no
dependencies at all. `store` reads and writes pages. `cli` adds the
binary and the clap surface. A tool that parses its own arguments turns
`cli` off and asks for text:

```rust
static DOCS: &[(&str, &str)] = diataxis::embedded_docs!();

fn docs(args: &[String]) -> diataxis::Result<()> {
    let request = diataxis::docs::request_from_args(args)?;
    print!("{}", diataxis::DocSet::from_embedded(DOCS)?.render(request)?);
    Ok(())
}
```

gaff is that tool. clap exits 2 on a usage error, and 2 is the code gaff
reserves for a guard refusing a tool call, so gaff parses its own
arguments and keeps its own exit codes.

Every adopting tool gains `docs --all`, which prints every page whole in
one stream, for an agent that wants one read rather than a listing and
four fetches.

## Related

- [tisket](https://github.com/cjohnhanson/tisket) — issue tracker. Markdown issues with YAML frontmatter, in the repository
- [zettel](https://github.com/cjohnhanson/zettel) — zettelkasten notes for a repository
- [almanac](https://github.com/cjohnhanson/almanac) — agent skill index, over pluggable sources
- [missouri](https://github.com/cjohnhanson/missouri) — model-based testing where a state is a directory
- [gaff](https://github.com/cjohnhanson/gaff) — context-lifecycle handler for coding agents
- [mdstore](https://github.com/cjohnhanson/mdstore) — the frontmattered markdown library underneath

Diátaxis itself is Daniele Procida's, at [diataxis.fr](https://diataxis.fr/),
under CC-BY-SA 4.0. This tool implements it; it does not define it.

## License

MIT.

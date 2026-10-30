# Changelog

## Unreleased

### Added

- `diataxis export --out <DIR>` writes every page of the set as
  front-matter Markdown for a static site generator such as Hugo.
  Each `<DIR>/<id>.md` file has `---` front matter with `title`,
  `description`, `weight`, and `params.diataxis_type`.
  Export omits `description` when empty. The type sits under `params`
  because Hugo reserves `type` for choosing a layout.

- `weight` orders pages by type: tutorials, then how-to guides, then
  reference, then explanation. Pages within a type sort by title,
  without regard to case. Weights start at 1.

- Export turns `[[page]]` into `[<target title>](<id>.md)`, with markup
  characters in the title escaped. `[[page|text]]` keeps its text.
  Export drops the leading H1 because the site renders the title
  from front matter.

- Export runs `check` first and refuses a set with a fault. It also
  refuses references to pages in another set, such as `[[alias:page]]`.
  A single-set export cannot turn these references into links.

- `<DIR>` must not exist. Export creates it and removes it again if
  a write fails, so a failed export leaves nothing behind.
  Export never writes into an existing directory such as the pages
  directory.

- Export leaves sidebar grouping, edit links, and the generator's
  reserved file names to the site's own configuration.

- `Repo::list_strict` returns an error on the first page that does
  not parse, where `list` skips it. `check::prose_refs` returns each
  prose reference with its byte range and optional text.

### Changed

- References use pulldown-cmark's wikilink extension, with one grammar
  for `check`, the closure of declared sets, and `export`.
  pulldown-cmark is now at least 0.13.6. `[[two words]]` is now a
  reference, and `check` reports it when it names no page.
  `[[page|text]]` is read with its text. An escaped `\[[x]]` stays
  literal text. A reference next to a `[x]: url` definition of the
  same name is still a reference.

- `docs/cli-reference.md` is generated from the clap definition with
  `usage-lib`. Each usage line comes from clap, so the page and
  `--help` show the same usage. A test fails when the committed page
  differs from what the definition renders.
  `UPDATE_CLI_REFERENCE=1 cargo test --test cli_reference` rewrites
  the page. `usage-lib` is a test-only dev-dependency; consumers
  never compile it. The test suite needs Rust 1.91.

- Long help on every command carries the details from the old
  hand-written reference page. It states that `init` fails when
  `diataxis.yml` exists and that `check` lists faults and advice.
  It identifies `store sync` as the only command that reaches the
  network. It also states that `--home` conflicts with `--root`
  and that `-V` works at the top level only.

## 0.1.1

### Fixed

- The lib compiles under every feature. 0.1.0 shipped a lib that did not
  build with `--no-default-features --features store`, which is how two
  of its five consumers take it. `TOOL` sat in the `cli` module and
  `workspace` read it with `cli` off.

- `check` gave wrong answers in both directions. It reported a reference
  inside an indented code block, and inside a fenced block within a
  blockquote, and it missed `[a](page.md "a title")`. Three hand-rolled
  scanners are one pulldown-cmark pass now.

- The cross-set link check never worked. It read the labels of
  unavailable sets and filtered them for a `:`, which they do not carry,
  so it returned nothing on every input and hid the unavailable set as
  well. It reports the set that did not load, which is what mdstore can
  answer.

- `run_command` refuses `store root` rather than panicking. The function
  and its argument types are public, so a consumer could abort its own
  process.

- `store root` prints the file it reads. It named
  `~/.config/mdstore/config.yml` while reading
  `~/.config/diataxis/config.yml`.

## 0.1.0

The first release. Broken: see 0.1.1.

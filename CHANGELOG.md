# Changelog

## Unreleased

### Added

- `diataxis export --out <DIR> [--edit-base <URL>]` writes every page to
  `<DIR>/<id>.md` with `---` YAML front matter for Hugo and similar
  static site generators: `title`, `description`, `weight`, and
  `params.diataxis_type`. Hugo reserves top-level `type` for choosing a
  layout. `--edit-base` adds `editURL` by joining the page's file name to
  the given URL.

- Export turns `[[page]]` references in prose into relative Markdown
  links, `[<target title>](<id>.md)`. References inside code spans and
  code blocks stay as written. Export drops the leading H1 because the
  generator renders the title from front matter.

- Each type with pages gets `<DIR>/separator-<type>.md`, a sidebar label
  listed but never rendered: "Tutorials", "How-to guides", "Reference",
  or "Explanation". Empty types get none. Weights put types in Diataxis
  order, each separator first in its group, and pages in title order.
  No weight is zero because Hugo reads zero as no weight.

- Export stops with an error on a page file that does not parse, a
  `[[reference]]` that names no page, or a `[[alias:page]]` reference to
  another set. `list` skips an unparsable page with a note on stderr;
  export refuses because a published site would silently lose the page.

- `Repo::list_strict` returns an error on the first page that does not
  parse, where `list` skips it.

### Changed

- `docs/cli-reference.md` is generated from the clap definition with
  `usage-lib`. A test fails when the committed page differs from what
  the definition renders.
  `UPDATE_CLI_REFERENCE=1 cargo test --test cli_reference` rewrites the
  page. `usage-lib` is a test-only dev-dependency; crate consumers never
  compile it.

- `diataxis <command> --help` includes the reference page's command
  details in long help. `init` documents its error when `diataxis.yml`
  exists. `check` lists which findings are faults and which are advice.
  `store sync` states that it is the only command that reaches the
  network.

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

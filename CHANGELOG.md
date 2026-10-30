# Changelog

## Unreleased

### Added

- `diataxis export --out <DIR> [--edit-base <URL>]` writes every page as
  Hugo content in `<DIR>/<id>.md`. Its `---` front matter contains
  `title`, `description`, `weight`, and `params.diataxis_type`.
  Export omits `description` when empty. The type sits under `params`
  because Hugo reserves `type` for choosing a layout. `--edit-base`
  adds `editURL` by joining the page's file name to the given URL.
  It accepts only an HTTP or HTTPS URL and refuses any other value.

- Export turns `[[page]]` references in prose into
  `[<target title>](<id>.md)`, with markup characters in the title
  escaped. References inside code or HTML, with an escaped opening
  bracket, or with emphasis inside the brackets stay as written.
  Export drops the leading H1 because the site renders the title
  from front matter.

- Each type with pages gets `<DIR>/separator-<type>.md`, a sidebar
  group label in the form the Hextra theme reads. The labels are
  "Tutorials", "How-to guides", "Reference", and "Explanation".
  These files are listed but never rendered. Weights put types in
  Diataxis order, each separator first, and pages in title order.
  No weight is zero.

- `<DIR>` must be empty or absent. A stale or hand-written file never
  sits among the exported pages. Export refuses the pages directory
  as `<DIR>` and changes nothing. Export renders every file before
  it writes one, so a fault leaves `<DIR>` as it was.
  Export rejects a page that does not parse, a reference that names
  no page, or a reference to another set. It also rejects a page
  whose id is a separator file name, `_index`, or `index`.
  Each error names the page or path and the fix.

- `Repo::list_strict` returns an error on the first page that does
  not parse, where `list` skips it. `check::prose_refs` returns each
  prose reference with its byte range.

### Changed

- `check` reads references with the same scan as `export`. Both read
  the same text the same way, so a set that passes `check` exports.
  An escaped `\[[x]]` and `[[*x*]]` are no longer reported as
  references.

- `docs/cli-reference.md` is generated from the clap definition with
  `usage-lib`. A test fails when the committed page differs from
  what the definition renders.
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

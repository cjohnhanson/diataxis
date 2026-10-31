# Changelog

## 0.2.0

### Added

- `diataxis export --out <DIR>` writes every page of the set as
  front-matter Markdown for a static site generator such as Hugo.
  Each `<DIR>/<id>.md` file has `---` front matter with `title`,
  `description`, `weight`, and `params.diataxis_type`. Export omits
  `description` when empty. The type sits under `params` because Hugo
  reserves `type` for choosing a layout.

- `weight` orders pages by type: tutorials, then how-to guides, then
  reference, then explanation. Export sorts pages within each type by
  title, without regard to case. Weights start at 1.

- Export turns `[[page]]` into `[<target title>](<page.md>)`.
  Links use angle brackets so an id may hold a space or a parenthesis.
  Export escapes markup characters in the target title.
  `[[page|text]]` keeps its text. Export drops the leading H1 because
  the site renders the title from front matter.

- Export runs `check` first and refuses a set with a fault. The error
  lists each fault, so no second command is needed.

- Export also refuses references to pages in another set,
  `[[alias:page]]`, and embeds, `![[page]]`. A single-set export cannot
  link them. One error names every such reference.

- `<DIR>` must not exist. Export creates it and removes it again if a
  write fails, so a failed export leaves nothing behind. It never
  writes into an existing directory, including the pages directory.
  To export again, remove the previous output first.

- Export leaves sidebar grouping, edit links, and reserved file names
  to the site. Hugo gives page ids `index` and `_index` special
  meaning; avoiding those ids is the site's choice.

- `Repo::list_strict` returns an error on the first page that does not
  parse, where `list` skips it. `check::prose_refs` returns each prose
  reference with its byte range, its optional text, and whether it is
  an embed.

### Changed

- pulldown-cmark's wikilink extension reads references with one grammar
  for `check`, the closure of declared sets, and `export`.
  pulldown-cmark is at least 0.13.6. `[[two words]]` is a reference,
  and `check` reports it when it names no page. The parser reads
  `[[page|text]]` with its text. `check` checks `![[page]]` like a
  reference. An escaped `\[[x]]` stays literal text. A reference next
  to a `[x]: url` definition of the same name remains a reference.

- `usage-lib` generates `docs/cli-reference.md` from the clap
  definition. Each usage line comes from clap, so the page and
  `--help` show the same usage. The page lists global options once,
  at the top. A test fails when the committed page differs.
  `UPDATE_CLI_REFERENCE=1 cargo test --test cli_reference` rewrites
  it. `usage-lib` is a test-only dev-dependency; consumers never
  compile it. The test suite needs Rust 1.91.

- Long help now includes facts previously confined to the reference
  page. It states that `init` fails when `diataxis.yml` exists and
  that `check` lists its faults and its advice. It identifies
  `store sync` as the only command that reaches the network.
  It also states that `--home` conflicts with `--root` and that
  `-V` works at the top level only.

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

# Changelog

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

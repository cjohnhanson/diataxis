<!-- metadata
title: "How to adopt diataxis in an existing project"
description: "Point a build script at the pages and drop the hand-written table"
type: how-to
-->

# How to adopt diataxis in an existing project

Use this when a project already has a `docs` directory and its own
`docs` subcommand backed by a hand-written table of pages.

## Declare the set

Write `diataxis.yml` beside `Cargo.toml`:

```yaml
docs_dir: docs
title: acme
```

Run the check to see what the pages say:

```sh
diataxis check
```

A page with no type, or with a type that is not one of the four, comes
back as a fault. Give it one by editing the file, in the frontmatter:

```yaml
---
title: Composition
type: explanation
---
```

`page edit` does not reach such a file. Every command that reads a page
reads its type first, so a file with no type is not yet a page to them.
Adding the field by hand is what makes it one, and `check` passes after.

## Embed the set

Take this crate as a build-dependency. The `build` feature carries no
dependencies of its own:

```toml
[build-dependencies]
diataxis = { version = "0.2", default-features = false, features = ["build"] }

[dependencies]
diataxis = { version = "0.2", default-features = false, features = ["store"] }
```

Add `build.rs`:

```rust
fn main() -> std::io::Result<()> {
    diataxis::build::embed_docs()
}
```

Read the generated array wherever the pages are needed:

```rust
static DOCS: &[(&str, &str)] = diataxis::embedded_docs!();
```

Nothing lists the pages. The build script reads `docs_dir` from
`diataxis.yml`, embeds every markdown file it holds, and tells cargo to
rebuild when one changes.

## Wire the subcommand

If the tool uses clap, take the `cli` feature and hand the arguments
straight over:

```rust
Command::Docs(args) => diataxis::DocSet::from_embedded(DOCS)?.run(&args),
```

If the tool parses its own arguments, ask for text and keep your own
exit codes:

```rust
let request = diataxis::docs::request_from_args(rest)?;
print!("{}", diataxis::DocSet::from_embedded(DOCS)?.render(request)?);
```

## Delete the old module

The hand-written table stated the title and the description that each
page already states, so the two could disagree. Remove it, along with
`list`, `show`, `search`, `find`, `find_in`, `format_list`,
`format_list_from_refs`, and `find_matching`.

## Check what a reader gets

```sh
acme docs              # one line per page
acme docs getting      # one page, by prefix
acme docs search pin   # the pages that hold a word
acme docs --all        # every page, for an agent that wants one read
```

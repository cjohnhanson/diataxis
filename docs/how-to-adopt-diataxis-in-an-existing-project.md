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

```yaml {filename="diataxis.yml"}
docs_dir: docs
title: acme
```

Run the check to see what the pages say:

```console
$ diataxis check
? 1
diataxis: skipping [CWD]/docs/composition.md: missing field `type`
fault  composition.md               not a page: missing field `type`
advice  the set                      no how-to pages
advice  the set                      no reference pages
advice  the set                      no explanation pages
Error:   x 1 fault(s)


```

A page with no type, or with a type that is not one of the four or an
alias `page create --help` lists, comes back as a fault. `page edit` does not reach such a file. Every command
that reads a page reads its type first, so a file with no type is not
yet a page to them:

```console
$ diataxis page edit composition -T explanation
? 1
diataxis: skipping [CWD]/docs/composition.md: missing field `type`
Error:   x no page matches composition


```

Give it a type by editing the file, in the frontmatter:

```markdown {filename="docs/composition.md"}
---
title: Composition
type: explanation
---

A job draws on other jobs.
```

Adding the field by hand is what makes it a page, and `check` passes
after:

```console
$ diataxis check
advice  the set                      no how-to pages
advice  the set                      no reference pages

```

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

```rust,no_run
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

If the tool uses clap, take the `cli` feature instead of `store`, and
hand the arguments straight over. `DocsArgs` holds the subcommand's
arguments:

```rust
static DOCS: &[(&str, &str)] = diataxis::embedded_docs!();

#[derive(clap::Subcommand)]
enum Command {
    /// Show the bundled documentation
    Docs(diataxis::DocsArgs),
}

fn run(command: Command) -> diataxis::Result<()> {
    match command {
        Command::Docs(args) => diataxis::DocSet::from_embedded(DOCS)?.run(&args),
    }
}
```

If the tool parses its own arguments, ask for text and keep your own
exit codes:

```rust
static DOCS: &[(&str, &str)] = diataxis::embedded_docs!();

fn docs(args: &[String]) -> diataxis::Result<()> {
    let request = diataxis::docs::request_from_args(args)?;
    print!("{}", diataxis::DocSet::from_embedded(DOCS)?.render(request)?);
    Ok(())
}
```

## Delete the old module

The hand-written table stated the title and the description that each
page already states, so the two could disagree. Remove it, along with
every function that reads it: the listing, the lookup of one page, and
the search. `DocSet` does each of those now.

## Check what a reader gets

diataxis builds its own `docs` subcommand this way, so it shows what a
reader of acme gets. With no topic, one line per page:

```console
$ diataxis docs
  getting-started                              Set up a documentation set and write the first page
  how-to-adopt-diataxis-in-an-existing-project Point a build script at the pages and drop the hand-written table
  cli-reference                                Every command, argument, and flag of the diataxis binary, generated from its definition
  what-is-diataxis                             Why a documentation set carries types, and how the tool reads them

```

`search` lists the pages that hold a word:

```console
$ diataxis docs search embed
  how-to-adopt-diataxis-in-an-existing-project Point a build script at the pages and drop the hand-written table
  cli-reference                                Every command, argument, and flag of the diataxis binary, generated from its definition

```

`acme docs getting` prints one page, by its slug, its title, or a
prefix that matches one slug. `acme docs --all` prints every page whole,
for an agent that wants one read.

# Contributing

## Setup

```sh
git clone https://github.com/cjohnhanson/diataxis
cd diataxis
cargo build
cargo test --workspace --all-features
```

The docs test needs a POSIX shell and these tools on `PATH`: `sh`,
`env`, `mkdir`, `cat`, `printenv` and `pwd`. The missouri suite under
`tests/missouri/` needs [missouri](https://github.com/cjohnhanson/missouri).

## What CI checks

`main` is protected. One required check, `gate`, decides a merge. It
runs on every pull request, and it runs the same commands the local git
hooks run:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo hack check --each-feature --no-dev-deps
cd tests/missouri && missouri run
```

After those, the check reads a review note from the head commit of the
branch. `.gaff/gaff.yml` names the reviews a change must pass
(`gaff reviews` prints them), and `.agents/skills/signoff-driver/SKILL.md`
has the procedure. As a contributor from outside, open the pull request
and stop there. The maintainer runs the reviews on its head commit and
writes the note.

`cargo test` in `gate` runs the docs tests described under
[Docs tests](#docs-tests). A second workflow, `docs`, runs on every pull
request and every push to `main`. It runs the family checks listed at
the end of that section. Pushes to `main` also check web links.

## Docs tests

`tests/docs.rs` runs every `console` block in `README.md` and `docs/*.md`
against the built binary with trycmd. Each page runs in its own scratch
directory with its own `HOME` and XDG directories, and a cleared
environment. `[CWD]` stands for the page's working directory, and
`[PARENT]` for the directory that holds it.

A fence such as `` ```yaml {filename="diataxis.yml"} `` declares a file
the reader writes. The test writes that file before running the
commands that follow it.

Fixtures for each page live in `tests/docs/<page>/`. `setup.md` holds
blocks that run before the page. The test copies `work/` into the working
directory and `home/` into `HOME`.

The same test compares each visible command's `--help` output with a file
in `tests/snapshots/help/`. `tests/cli_reference.rs` generates
`docs/cli-reference.md` from the same doc comments in `src/cli.rs`.
Change help text in those comments, never in a snapshot or the generated
page.

The `rust` blocks in `README.md` and in the adoption guide compile as
doctests (`cargo test --doc --all-features`). A `toml` dependency line
must name this crate's version and features that exist; `tests/docs.rs`
checks both.

These commands write the current output to the expected files:

```sh
TRYCMD=overwrite cargo test --test docs
SNAPSHOTS=overwrite cargo test --test docs
UPDATE_CLI_REFERENCE=1 cargo test --test cli_reference
```

Read the diff after running them.

In `README.md` and `docs/*.md`, put every diataxis command in a `console`
block, except under `## Install`. A `sh`, `text` or other tagged block
never runs. Give every fence a language: trycmd runs fences that have
none.

The `docs` workflow calls `.github/workflows/family-checks.yml` in this
repository. The same checks run for diataxis, tundish, gaff, tisket,
zettel, almanac and missouri. They read `README.md` and `docs/` and fail
when:

- `tests/docs.rs` is missing or does not name trycmd, `README.md` and
  `docs`.
- A command for any of those tools appears outside a `console` block,
  except under Install.
- A `console` block does not start with a `$ ` line.
- A fence has no language or uses the `trycmd` language.
- The README's first heading is not a level-1 heading that names
  `diataxis` as a word.
- The README has a second level-1 heading or no paragraph before its
  first section.
- The README has no `## Install` section or does not end with
  `## License`.
- A link to a file or heading in the repository is broken.
- `typos` finds a misspelling.

The scripts and their rules live under `scripts/family/`. Each script
prints its rules with `--help`.

## Pull requests

Branch from `main` and open the pull request from a fork. Keep a change
and its tests in one pull request.

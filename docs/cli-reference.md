<!-- metadata
title: "diataxis CLI Reference"
description: "Every command, argument, and option of the diataxis binary"
type: reference
-->

# diataxis CLI Reference

## Global options

Every command that reads or writes a documentation set takes these.

| Option | Effect |
|---|---|
| `--root <ROOT>` | Act on this directory. Literal: it must hold `diataxis.yml`. No walk, no fallback. |
| `--home` | Act on the configured root set, wherever the command runs. Conflicts with `--root`. |
| `-h`, `--help` | Print help. |
| `-V`, `--version` | Print the version. |

Without `--root` or `--home`, a command uses the nearest `diataxis.yml`
at or above the working directory. With none, a read falls back to the
configured root set and names it on stderr.

## init

```
diataxis init
```

Writes `diataxis.yml` and makes the pages directory. An existing
directory of pages is adopted, so a repository with docs already
written runs `init` once and keeps every page.

Fails when `diataxis.yml` exists.

## page create

```
diataxis page create [OPTIONS] --type <TYPE> <TITLE>
```

| Argument | Effect |
|---|---|
| `<TITLE>` | The page title. A how-to title states a goal: "How to ...". |
| `-T`, `--type <TYPE>` | `tutorial`, `how-to`, `reference`, or `explanation`. |
| `-d`, `--description <TEXT>` | The one-line description. |
| `-t`, `--tags <TAGS>` | Tags, separated by commas. |
| `-b`, `--body <TEXT>` | The page body. |
| `--id <ID>` | The file stem, when the slug from the title is not wanted. |

The type also accepts `guide`, `howto`, `how_to`, and `how-to-guide`
for a how-to guide, and `discussion` or `background` for an
explanation. Thirteen pages in four repositories were written with
`guide` before this tool existed.

Fails when a page of that id already exists.

## page list

```
diataxis page list [-T <TYPE>]
```

One line per page: the id, the type, and the title. `-T` shows one type
only.

## page show

```
diataxis page show <PAGE>
```

Prints the metadata and the body. `<PAGE>` is a page id or a unique
prefix of one. An ambiguous prefix is an error that names what it
matched.

## page edit

```
diataxis page edit [OPTIONS] <PAGE>
```

| Option | Effect |
|---|---|
| `--title <TEXT>` | Replace the title. |
| `-d`, `--description <TEXT>` | Replace the description. |
| `-T`, `--type <TYPE>` | Retype the page. |
| `-t`, `--tags <TAGS>` | Replace the tags. |
| `-b`, `--body <TEXT>` | Replace the body. |

A field left out is left alone. The page is written back in the fence
form it was read in, so an edit never converts a file.

## page delete

```
diataxis page delete <PAGE>
```

Removes the file.

## page path

```
diataxis page path <PAGE>
```

Prints the file path. Use it to open a page in an editor.

## map

```
diataxis map
```

Prints the set as the four quadrants, in Diátaxis order: tutorial,
how-to, reference, explanation. Each names what it informs and what it
serves, with its pages under it. A type with no pages prints `(none)`.

## compass

```
diataxis compass [<PAGE>]
```

Prints the two questions that name a type, and the table that turns a
pair of answers into one. Given a page, it prints that page's declared
type alongside.

The compass does not classify a page. Classification needs a reader,
and a tool that guessed would be inventing a fact.

Without a page, the compass resolves no set and runs from any
directory. An agent recalls the two questions without standing in one.

## check

```
diataxis check
```

Reports two kinds of finding, faults first. A fault is provable from
the files. Advice is a gap worth seeing that is not wrong on its own.

| Finding | Severity |
|---|---|
| A markdown file in the pages directory that is not a page | fault |
| A how-to title that does not state a goal | fault |
| A `[[reference]]` that names no page | fault |
| A relative `.md` link that names no file | fault |
| A reference into a declared set that names no page there | fault |
| A type with no pages | advice |

Exits non-zero when a fault is found. A reference inside a code span or
a fenced block is not a link, so documentation that quotes syntax is
not reported.

Nothing here judges prose. Whether an explanation has crept into a
reference page needs a reader.

## search

```
diataxis search <PATTERN>
```

A regular expression, matched against every page body without regard to
case. Prints the page id, the line number, the type, and the line.

## store list

```
diataxis store list
```

One row per set in the closure: this set first, then each declared set
under its alias, with its source and its page count. A remote set shows
the time since its last fetch.

## store sync

```
diataxis store sync
```

Fetches every declared remote set into the local cache. Exits non-zero
when a fetch fails. No other command reaches the network.

## store root

```
diataxis store root [<PATH>] [--force]
```

Without a path, prints the configured root set. With one, sets it. The
directory must hold `diataxis.yml`. Changing a root that is already set
needs `--force`.

This command resolves nothing and runs from any directory.

## docs

```
diataxis docs [<TOPIC>] [<QUERY>]
diataxis docs --all
```

| Form | Effect |
|---|---|
| `docs` | One line per bundled page. |
| `docs list` | The same listing. |
| `docs <topic>` | One page, by slug, title, or unique prefix. |
| `docs search <query>` | The pages that hold the query. |
| `docs --all` | Every page whole, in one stream. |

The pages are compiled into the binary, so `docs` works from any
directory with no files present.

## prime

```
diataxis prime
```

Prints what diataxis is and how to use it, for an agent's context. It
depends only on the binary, so it runs anywhere.

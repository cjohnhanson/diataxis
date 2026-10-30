<!-- metadata
title: diataxis CLI reference
description: Every command, argument, and flag of the diataxis binary, generated from its definition
type: reference
-->

# diataxis CLI reference

- **Version:** 0.1.1

Diataxis documentation sets as typed markdown


- **Usage:** `diataxis [--root <ROOT>] [--home] <SUBCOMMAND>`

## Global Flags

### `--root <ROOT>`

Documentation set directory. Literal: the directory must hold diataxis.yml; no walk, no fallback. Without it, the nearest diataxis.yml at or above the cwd is used.

An empty value is refused. It read as a relative path, joined to nothing, and acted on the working directory instead, so an unset variable wrote to whichever set the shell happened to stand in.

### `--home`

Act on the configured root store, wherever the command runs.

Without `--root` or `--home`, a command uses the nearest `diataxis.yml` at or above the working directory. With none, a read falls back to the configured root set and names it on stderr.

## `diataxis init`

- **Usage:** `diataxis init`

Initialize a documentation set in the current directory

Writes `diataxis.yml` and makes the pages directory. An existing directory of pages is adopted, so a repository with docs already written runs `init` once and keeps every page.

Fails when `diataxis.yml` exists.

## `diataxis prime`

- **Usage:** `diataxis prime`

Print what diataxis is and how to use it, for an agent's context

Depends only on the binary, so it runs from any directory.

## `diataxis page`

- **Usage:** `diataxis page <SUBCOMMAND>`

Manage pages

## `diataxis page create`

- **Usage:** `diataxis page create <FLAGS> <TITLE>`

Create a page of one type

The type also accepts `guide`, `howto`, `how_to`, and `how-to-guide` for a how-to guide, and `discussion` or `background` for an explanation.

Fails when a page of that id already exists.

### Arguments

#### `<TITLE>`

The page title. A how-to title states a goal: "How to ..."

### Flags

#### `-T --type <TYPE>`

The Diataxis type: tutorial, how-to, reference, or explanation

#### `-d --description <DESCRIPTION>`

The one-line description

#### `-t --tags <TAGS>`

Tags, separated by commas

#### `-b --body <BODY>`

The page body

#### `--id <ID>`

The file stem, when the slug from the title is not wanted

## `diataxis page list`

- **Usage:** `diataxis page list [-T --type <TYPE>]`

List the pages

One line per page: the id, the type, and the title.

### Flags

#### `-T --type <TYPE>`

Show one type only

## `diataxis page show`

- **Usage:** `diataxis page show <PAGE>`

Show one page

Prints the metadata and the body. An ambiguous prefix is an error that names what it matched.

### Arguments

#### `<PAGE>`

The page id, or a unique prefix of one

## `diataxis page edit`

- **Usage:** `diataxis page edit [FLAGS] <PAGE>`

Change a page

A field left out is left alone. The page is written back in the metadata form it was read in, so an edit never converts a file.

### Arguments

#### `<PAGE>`

The page id, or a unique prefix of one

### Flags

#### `--title <TITLE>`

Replace the title

#### `-d --description <DESCRIPTION>`

Replace the description

#### `-T --type <TYPE>`

Retype the page

#### `-t --tags <TAGS>`

Replace the tags

#### `-b --body <BODY>`

Replace the body

## `diataxis page delete`

- **Usage:** `diataxis page delete <PAGE>`

Remove a page

### Arguments

#### `<PAGE>`

The page id, or a unique prefix of one

## `diataxis page path`

- **Usage:** `diataxis page path <PAGE>`

Print the file path of a page, to open it in an editor

### Arguments

#### `<PAGE>`

The page id, or a unique prefix of one

## `diataxis map`

- **Usage:** `diataxis map`

Show the set as the four Diataxis quadrants

Prints the quadrants in Diataxis order: tutorial, how-to, reference, explanation. Each names what it informs and what it serves, with its pages under it. A type with no pages prints `(none)`.

## `diataxis compass`

- **Usage:** `diataxis compass [PAGE]`

Show the two questions that name a page's type

Prints the two questions and the table that turns a pair of answers into one type. Given a page, it prints that page's declared type alongside.

The compass does not classify a page: classification needs a reader. Without a page, it resolves no set and runs from any directory.

### Arguments

#### `[PAGE]`

A page to name alongside the questions

## `diataxis check`

- **Usage:** `diataxis check`

Check the pages for type faults and broken links

Faults print first. A fault is provable from the files: a Markdown file in the pages directory that is not a page, a how-to title that does not state a goal, a `[[reference]]` that names no page, a relative `.md` link that names no file, and a declared set that did not load. Advice is a gap that is not wrong on its own: a type with no pages.

Exits non-zero when a fault is found. A reference inside a code span or a code block is not a link. Nothing here judges prose.

## `diataxis search`

- **Usage:** `diataxis search <PATTERN>`

Search the pages

Matches a regular expression against every page body, without regard to case. Prints the page id, the line number, the type, and the line.

### Arguments

#### `<PATTERN>`

A regular expression, matched without regard to case

## `diataxis export`

- **Usage:** `diataxis export <--out <OUT>> [--edit-base <EDIT_BASE>]`

Write the pages as front-matter Markdown for a static site generator

Each page becomes `OUT/<id>.md` with `---` front matter: `title`, `description`, `weight`, `params.diataxis_type`, and `editURL` when `--edit-base` is given. A `[[page]]` reference in prose becomes a relative `<id>.md` link; a reference in code stays as written. The leading H1 is dropped. Each type that has pages also gets `OUT/separator-<type>.md`, a sidebar group label.

Fails on a page that does not parse, a reference that names no page, and a reference to another set.

### Flags

#### `--out <OUT>`

The directory to write into. It is created if absent

#### `--edit-base <EDIT_BASE>`

The URL the page files sit under, for an "edit this page" link, such as https://github.com/acme/tool/edit/main/docs

## `diataxis store`

- **Usage:** `diataxis store <SUBCOMMAND>`

Manage the declared documentation sets

## `diataxis store list`

- **Usage:** `diataxis store list`

List the documentation sets this set reads

One row per set in the closure: this set first, then each declared set under its alias, with its source and its page count. A remote set shows the time since its last fetch.

## `diataxis store sync`

- **Usage:** `diataxis store sync`

Fetch the declared remote sets into the local cache

Exits non-zero when a fetch fails. No other command reaches the network.

## `diataxis store root`

- **Usage:** `diataxis store root [--force] [PATH]`

Show or set the root set that reads fall back to

The directory must hold `diataxis.yml`. This command resolves no set and runs from any directory.

### Arguments

#### `[PATH]`

The directory of the root set. Without it, print the current setting

### Flags

#### `--force`

Replace an already-set root with a different one

## `diataxis docs`

- **Usage:** `diataxis docs [-a --all] [TOPIC] [QUERY]`

Show the bundled documentation

With no topic, or with `list`, prints one line per bundled page. A topic prints one page, by slug, title, or unique prefix. `search <query>` prints the pages that hold the query. `--all` prints every page whole, in one stream. The pages are compiled into the binary, so this works from any directory.

### Arguments

#### `[TOPIC]`

Topic slug to show, or "search" to search the docs

#### `[QUERY]`

Search query (when topic is "search")

### Flags

#### `-a --all`

Print every page whole, in one stream

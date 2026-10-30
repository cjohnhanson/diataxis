<!-- metadata
title: diataxis CLI reference
description: Every command, argument, and flag of the diataxis binary, generated from its definition
type: reference
-->

# diataxis CLI reference

- **Version:** 0.1.1

Diataxis documentation sets as typed markdown

Every command takes `-h` or `--help`. `-V` and `--version` work at the
top level only, as `diataxis --version`; a subcommand refuses them.

- **Usage:** `diataxis [OPTIONS] <COMMAND>`

## Global Flags

### `--root <ROOT>`

Documentation set directory. Literal: the directory must hold diataxis.yml; no walk, no fallback. Without it, the nearest diataxis.yml at or above the cwd is used.

An empty value is refused.

### `--home`

Act on the configured root set, wherever the command runs.

Conflicts with `--root`. Without either, a read outside any set falls back to the configured root set and names it on stderr.

**Default:** `false`

## Flags

### `-h --help`

Print help (see a summary with '-h')

### `-V --version`

Print version

## `diataxis init`

- **Usage:** `diataxis init [OPTIONS]`

Initialize a documentation set in the current directory

Writes `diataxis.yml` and makes the pages directory. An existing directory of pages is adopted, so a repository with docs already written runs `init` once and keeps every page.

Fails when `diataxis.yml` exists.

### Flags

#### `--root <ROOT>`

Documentation set directory. Literal: the directory must hold diataxis.yml; no walk, no fallback. Without it, the nearest diataxis.yml at or above the cwd is used.

An empty value is refused.

#### `--home`

Act on the configured root set, wherever the command runs.

Conflicts with `--root`. Without either, a read outside any set falls back to the configured root set and names it on stderr.

**Default:** `false`

#### `-h --help`

Print help (see a summary with '-h')

## `diataxis prime`

- **Usage:** `diataxis prime [OPTIONS]`

Print what diataxis is and how to use it, for an agent's context

Depends only on the binary, so it runs from any directory.

### Flags

#### `--root <ROOT>`

Documentation set directory. Literal: the directory must hold diataxis.yml; no walk, no fallback. Without it, the nearest diataxis.yml at or above the cwd is used.

An empty value is refused.

#### `--home`

Act on the configured root set, wherever the command runs.

Conflicts with `--root`. Without either, a read outside any set falls back to the configured root set and names it on stderr.

**Default:** `false`

#### `-h --help`

Print help (see a summary with '-h')

## `diataxis page`

- **Usage:** `diataxis page [OPTIONS] <COMMAND>`

Manage pages

### Global Flags

#### `--root <ROOT>`

Documentation set directory. Literal: the directory must hold diataxis.yml; no walk, no fallback. Without it, the nearest diataxis.yml at or above the cwd is used.

An empty value is refused.

#### `--home`

Act on the configured root set, wherever the command runs.

Conflicts with `--root`. Without either, a read outside any set falls back to the configured root set and names it on stderr.

**Default:** `false`

### Flags

#### `-h --help`

Print help (see a summary with '-h')

## `diataxis page create`

- **Usage:** `diataxis page create [OPTIONS] --type <TYPE> <TITLE>`

Create a page of one type

The type also accepts `guide`, `howto`, `how_to`, and `how-to-guide` for a how-to guide, and `discussion` or `background` for an explanation.

Fails when a page of that id already exists.

### Arguments

#### `<TITLE>`

The page title. A how-to title states a goal: "How to ..."

### Flags

#### `-T --type <TYPE>`

The Diataxis type: tutorial, how-to, reference, or explanation

#### `--root <ROOT>`

Documentation set directory. Literal: the directory must hold diataxis.yml; no walk, no fallback. Without it, the nearest diataxis.yml at or above the cwd is used.

An empty value is refused.

#### `-d --description <DESCRIPTION>`

The one-line description

#### `--home`

Act on the configured root set, wherever the command runs.

Conflicts with `--root`. Without either, a read outside any set falls back to the configured root set and names it on stderr.

**Default:** `false`

#### `-t --tags <TAGS>`

Tags, separated by commas

#### `-b --body <BODY>`

The page body

#### `--id <ID>`

The file stem, when the slug from the title is not wanted

#### `-h --help`

Print help (see a summary with '-h')

## `diataxis page list`

- **Usage:** `diataxis page list [OPTIONS]`

List the pages

One line per page: the id, the type, and the title.

### Flags

#### `-T --type <TYPE>`

Show one type only

#### `--root <ROOT>`

Documentation set directory. Literal: the directory must hold diataxis.yml; no walk, no fallback. Without it, the nearest diataxis.yml at or above the cwd is used.

An empty value is refused.

#### `--home`

Act on the configured root set, wherever the command runs.

Conflicts with `--root`. Without either, a read outside any set falls back to the configured root set and names it on stderr.

**Default:** `false`

#### `-h --help`

Print help (see a summary with '-h')

## `diataxis page show`

- **Usage:** `diataxis page show [OPTIONS] <PAGE>`

Show one page

Prints the metadata and the body. An ambiguous prefix is an error that names what it matched.

### Arguments

#### `<PAGE>`

The page id, or a unique prefix of one

### Flags

#### `--root <ROOT>`

Documentation set directory. Literal: the directory must hold diataxis.yml; no walk, no fallback. Without it, the nearest diataxis.yml at or above the cwd is used.

An empty value is refused.

#### `--home`

Act on the configured root set, wherever the command runs.

Conflicts with `--root`. Without either, a read outside any set falls back to the configured root set and names it on stderr.

**Default:** `false`

#### `-h --help`

Print help (see a summary with '-h')

## `diataxis page edit`

- **Usage:** `diataxis page edit [OPTIONS] <PAGE>`

Change a page

A field left out is left alone. The page is written back in the metadata form it was read in, so an edit never converts a file.

### Arguments

#### `<PAGE>`

The page id, or a unique prefix of one

### Flags

#### `--title <TITLE>`

Replace the title

#### `--root <ROOT>`

Documentation set directory. Literal: the directory must hold diataxis.yml; no walk, no fallback. Without it, the nearest diataxis.yml at or above the cwd is used.

An empty value is refused.

#### `-d --description <DESCRIPTION>`

Replace the description

#### `--home`

Act on the configured root set, wherever the command runs.

Conflicts with `--root`. Without either, a read outside any set falls back to the configured root set and names it on stderr.

**Default:** `false`

#### `-T --type <TYPE>`

Retype the page

#### `-t --tags <TAGS>`

Replace the tags

#### `-b --body <BODY>`

Replace the body

#### `-h --help`

Print help (see a summary with '-h')

## `diataxis page delete`

- **Usage:** `diataxis page delete [OPTIONS] <PAGE>`

Remove a page

### Arguments

#### `<PAGE>`

The page id, or a unique prefix of one

### Flags

#### `--root <ROOT>`

Documentation set directory. Literal: the directory must hold diataxis.yml; no walk, no fallback. Without it, the nearest diataxis.yml at or above the cwd is used.

An empty value is refused.

#### `--home`

Act on the configured root set, wherever the command runs.

Conflicts with `--root`. Without either, a read outside any set falls back to the configured root set and names it on stderr.

**Default:** `false`

#### `-h --help`

Print help (see a summary with '-h')

## `diataxis page path`

- **Usage:** `diataxis page path [OPTIONS] <PAGE>`

Print the file path of a page, to open it in an editor

### Arguments

#### `<PAGE>`

The page id, or a unique prefix of one

### Flags

#### `--root <ROOT>`

Documentation set directory. Literal: the directory must hold diataxis.yml; no walk, no fallback. Without it, the nearest diataxis.yml at or above the cwd is used.

An empty value is refused.

#### `--home`

Act on the configured root set, wherever the command runs.

Conflicts with `--root`. Without either, a read outside any set falls back to the configured root set and names it on stderr.

**Default:** `false`

#### `-h --help`

Print help (see a summary with '-h')

## `diataxis page help`

- **Usage:** `diataxis page help [COMMAND]`

Print this message or the help of the given subcommand(s)

## `diataxis page help create`

- **Usage:** `diataxis page help create`

Create a page of one type

## `diataxis page help list`

- **Usage:** `diataxis page help list`

List the pages

## `diataxis page help show`

- **Usage:** `diataxis page help show`

Show one page

## `diataxis page help edit`

- **Usage:** `diataxis page help edit`

Change a page

## `diataxis page help delete`

- **Usage:** `diataxis page help delete`

Remove a page

## `diataxis page help path`

- **Usage:** `diataxis page help path`

Print the file path of a page, to open it in an editor

## `diataxis page help help`

- **Usage:** `diataxis page help help`

Print this message or the help of the given subcommand(s)

## `diataxis map`

- **Usage:** `diataxis map [OPTIONS]`

Show the set as the four Diataxis quadrants

Prints the quadrants in Diataxis order: tutorial, how-to, reference, explanation. Each names what it informs and what it serves, with its pages under it. A type with no pages prints `(none)`.

### Flags

#### `--root <ROOT>`

Documentation set directory. Literal: the directory must hold diataxis.yml; no walk, no fallback. Without it, the nearest diataxis.yml at or above the cwd is used.

An empty value is refused.

#### `--home`

Act on the configured root set, wherever the command runs.

Conflicts with `--root`. Without either, a read outside any set falls back to the configured root set and names it on stderr.

**Default:** `false`

#### `-h --help`

Print help (see a summary with '-h')

## `diataxis compass`

- **Usage:** `diataxis compass [OPTIONS] [PAGE]`

Show the two questions that name a page's type

Prints the two questions and the table that turns a pair of answers into one type. Given a page, it prints that page's declared type alongside.

The compass does not classify a page: classification needs a reader. Without a page, it resolves no set and runs from any directory.

### Arguments

#### `[PAGE]`

A page to name alongside the questions

### Flags

#### `--root <ROOT>`

Documentation set directory. Literal: the directory must hold diataxis.yml; no walk, no fallback. Without it, the nearest diataxis.yml at or above the cwd is used.

An empty value is refused.

#### `--home`

Act on the configured root set, wherever the command runs.

Conflicts with `--root`. Without either, a read outside any set falls back to the configured root set and names it on stderr.

**Default:** `false`

#### `-h --help`

Print help (see a summary with '-h')

## `diataxis check`

- **Usage:** `diataxis check [OPTIONS]`

Check the pages for type faults and broken links

Faults print first. A fault is provable from the files:

A Markdown file in the pages directory that is not a page.

A how-to title that does not state a goal.

A `[[reference]]` that names no page in this set.

A relative `.md` link that names no file.

A declared set that did not load.

Advice is a gap that is not wrong on its own: a type with no pages. Exits non-zero when a fault is found. A reference inside code, or escaped as `\[[x]]`, is literal text. Nothing here judges prose.

### Flags

#### `--root <ROOT>`

Documentation set directory. Literal: the directory must hold diataxis.yml; no walk, no fallback. Without it, the nearest diataxis.yml at or above the cwd is used.

An empty value is refused.

#### `--home`

Act on the configured root set, wherever the command runs.

Conflicts with `--root`. Without either, a read outside any set falls back to the configured root set and names it on stderr.

**Default:** `false`

#### `-h --help`

Print help (see a summary with '-h')

## `diataxis search`

- **Usage:** `diataxis search [OPTIONS] <PATTERN>`

Search the pages

Matches a regular expression against every page body, without regard to case. Prints the page id, the line number, the type, and the line.

### Arguments

#### `<PATTERN>`

A regular expression, matched without regard to case

### Flags

#### `--root <ROOT>`

Documentation set directory. Literal: the directory must hold diataxis.yml; no walk, no fallback. Without it, the nearest diataxis.yml at or above the cwd is used.

An empty value is refused.

#### `--home`

Act on the configured root set, wherever the command runs.

Conflicts with `--root`. Without either, a read outside any set falls back to the configured root set and names it on stderr.

**Default:** `false`

#### `-h --help`

Print help (see a summary with '-h')

## `diataxis export`

- **Usage:** `diataxis export [OPTIONS] --out <OUT>`

Write the pages as front-matter Markdown for a site generator

Each page becomes `OUT/<id>.md` with `---` front matter: `title`, `description`, `weight`, and `params.diataxis_type`. The weight orders pages by type, then by title. A `[[page]]` reference becomes a relative `<id>.md` link. The leading H1 is dropped.

Export runs `check` first and refuses a set with a fault. It also refuses a reference to a page in another set. `OUT` must not exist: export creates it, and removes it again if a write fails.

### Flags

#### `--out <OUT>`

The directory to create and write into; it must not exist yet

#### `--root <ROOT>`

Documentation set directory. Literal: the directory must hold diataxis.yml; no walk, no fallback. Without it, the nearest diataxis.yml at or above the cwd is used.

An empty value is refused.

#### `--home`

Act on the configured root set, wherever the command runs.

Conflicts with `--root`. Without either, a read outside any set falls back to the configured root set and names it on stderr.

**Default:** `false`

#### `-h --help`

Print help (see a summary with '-h')

## `diataxis store`

- **Usage:** `diataxis store [OPTIONS] <COMMAND>`

Manage the declared documentation sets

### Global Flags

#### `--root <ROOT>`

Documentation set directory. Literal: the directory must hold diataxis.yml; no walk, no fallback. Without it, the nearest diataxis.yml at or above the cwd is used.

An empty value is refused.

#### `--home`

Act on the configured root set, wherever the command runs.

Conflicts with `--root`. Without either, a read outside any set falls back to the configured root set and names it on stderr.

**Default:** `false`

### Flags

#### `-h --help`

Print help (see a summary with '-h')

## `diataxis store list`

- **Usage:** `diataxis store list [OPTIONS]`

List the documentation sets this set reads

One row per set in the closure: this set first, then each declared set under its alias, with its source and its page count. A remote set shows the time since its last fetch.

### Flags

#### `--root <ROOT>`

Documentation set directory. Literal: the directory must hold diataxis.yml; no walk, no fallback. Without it, the nearest diataxis.yml at or above the cwd is used.

An empty value is refused.

#### `--home`

Act on the configured root set, wherever the command runs.

Conflicts with `--root`. Without either, a read outside any set falls back to the configured root set and names it on stderr.

**Default:** `false`

#### `-h --help`

Print help (see a summary with '-h')

## `diataxis store sync`

- **Usage:** `diataxis store sync [OPTIONS]`

Fetch the declared remote sets into the local cache

Exits non-zero when a fetch fails. No other command reaches the network.

### Flags

#### `--root <ROOT>`

Documentation set directory. Literal: the directory must hold diataxis.yml; no walk, no fallback. Without it, the nearest diataxis.yml at or above the cwd is used.

An empty value is refused.

#### `--home`

Act on the configured root set, wherever the command runs.

Conflicts with `--root`. Without either, a read outside any set falls back to the configured root set and names it on stderr.

**Default:** `false`

#### `-h --help`

Print help (see a summary with '-h')

## `diataxis store root`

- **Usage:** `diataxis store root [OPTIONS] [PATH]`

Show or set the root set that reads fall back to

The directory must hold `diataxis.yml`. This command resolves no set and runs from any directory.

### Arguments

#### `[PATH]`

The directory of the root set. Without it, print the current setting

### Flags

#### `--force`

Replace an already-set root with a different one

**Default:** `false`

#### `--root <ROOT>`

Documentation set directory. Literal: the directory must hold diataxis.yml; no walk, no fallback. Without it, the nearest diataxis.yml at or above the cwd is used.

An empty value is refused.

#### `--home`

Act on the configured root set, wherever the command runs.

Conflicts with `--root`. Without either, a read outside any set falls back to the configured root set and names it on stderr.

**Default:** `false`

#### `-h --help`

Print help (see a summary with '-h')

## `diataxis store help`

- **Usage:** `diataxis store help [COMMAND]`

Print this message or the help of the given subcommand(s)

## `diataxis store help list`

- **Usage:** `diataxis store help list`

List the documentation sets this set reads

## `diataxis store help sync`

- **Usage:** `diataxis store help sync`

Fetch the declared remote sets into the local cache

## `diataxis store help root`

- **Usage:** `diataxis store help root`

Show or set the root set that reads fall back to

## `diataxis store help help`

- **Usage:** `diataxis store help help`

Print this message or the help of the given subcommand(s)

## `diataxis docs`

- **Usage:** `diataxis docs [OPTIONS] [TOPIC] [QUERY]`

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

**Default:** `false`

#### `--root <ROOT>`

Documentation set directory. Literal: the directory must hold diataxis.yml; no walk, no fallback. Without it, the nearest diataxis.yml at or above the cwd is used.

An empty value is refused.

#### `--home`

Act on the configured root set, wherever the command runs.

Conflicts with `--root`. Without either, a read outside any set falls back to the configured root set and names it on stderr.

**Default:** `false`

#### `-h --help`

Print help (see a summary with '-h')

## `diataxis help`

- **Usage:** `diataxis help [COMMAND]`

Print this message or the help of the given subcommand(s)

## `diataxis help init`

- **Usage:** `diataxis help init`

Initialize a documentation set in the current directory

## `diataxis help prime`

- **Usage:** `diataxis help prime`

Print what diataxis is and how to use it, for an agent's context

## `diataxis help page`

- **Usage:** `diataxis help page [COMMAND]`

Manage pages

## `diataxis help page create`

- **Usage:** `diataxis help page create`

Create a page of one type

## `diataxis help page list`

- **Usage:** `diataxis help page list`

List the pages

## `diataxis help page show`

- **Usage:** `diataxis help page show`

Show one page

## `diataxis help page edit`

- **Usage:** `diataxis help page edit`

Change a page

## `diataxis help page delete`

- **Usage:** `diataxis help page delete`

Remove a page

## `diataxis help page path`

- **Usage:** `diataxis help page path`

Print the file path of a page, to open it in an editor

## `diataxis help map`

- **Usage:** `diataxis help map`

Show the set as the four Diataxis quadrants

## `diataxis help compass`

- **Usage:** `diataxis help compass`

Show the two questions that name a page's type

## `diataxis help check`

- **Usage:** `diataxis help check`

Check the pages for type faults and broken links

## `diataxis help search`

- **Usage:** `diataxis help search`

Search the pages

## `diataxis help export`

- **Usage:** `diataxis help export`

Write the pages as front-matter Markdown for a site generator

## `diataxis help store`

- **Usage:** `diataxis help store [COMMAND]`

Manage the declared documentation sets

## `diataxis help store list`

- **Usage:** `diataxis help store list`

List the documentation sets this set reads

## `diataxis help store sync`

- **Usage:** `diataxis help store sync`

Fetch the declared remote sets into the local cache

## `diataxis help store root`

- **Usage:** `diataxis help store root`

Show or set the root set that reads fall back to

## `diataxis help docs`

- **Usage:** `diataxis help docs`

Show the bundled documentation

## `diataxis help help`

- **Usage:** `diataxis help help`

Print this message or the help of the given subcommand(s)

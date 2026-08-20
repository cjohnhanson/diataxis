<!-- metadata
title: "What is diataxis?"
description: "Why a documentation set carries types, and how the tool reads them"
type: explanation
-->

# What is diataxis?

Diataxis manages a documentation set as typed markdown. Every page
declares one of four types in its frontmatter: tutorial, how-to,
reference, or explanation. Every command reads that field.

## Why a type belongs in the file

The four types come from Daniele Procida's Diátaxis framework. Two
questions separate them. Does a page inform action or cognition? Does it
serve a reader acquiring skill, or one applying it. Four answers, four
forms, and the framework claims no fifth territory exists.

A documentation set that keeps the four apart serves each reader. A set
that lets them blur serves none. A tutorial that stops to explain loses
the learner it was written for. A reference page that grows an example
into an essay stops being consultable.

Nothing enforces the separation on its own. A page can claim one type
and hold another, and a documentation set of thirty pages hides the
drift. The type field makes the claim explicit, and `diataxis check`
tests what a machine can test about it.

## What the tool proves, and what it does not

A check is only worth reading if every finding is true. Diataxis
reports what the files prove: a page that does not parse, a page whose
type is not one of the four, a link that resolves to nothing, a how-to
title that states no goal.

It does not judge prose. Whether an explanation has crept into a
reference page needs a reader, and a check that guessed would fill the
report with confident noise. The compass prints the two questions for
that reader instead of answering them.

## Where a page lives

Pages live in the directory `diataxis.yml` names, `docs` by default.
Documentation is the published artefact, unlike an issue or a note, so
it stays where readers and site builds look for it.

A page id is its file stem. There is no random prefix, because the id
becomes a published path and a reader bookmarks it.

## The storage layer

Diataxis stores pages with mdstore, the same library under tisket,
zettel, and almanac. A page is a markdown document with frontmatter, and
frontmatter comes in two forms. The `---` form is what most markdown
tools expect. The comment form, `<!-- metadata` to `-->`, hides the same
keys from every markdown renderer, so a page read on a git forge shows
no metadata block. Diataxis reads both and writes the comment form.

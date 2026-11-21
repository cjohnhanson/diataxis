<!-- metadata
title: "Getting Started with diataxis"
description: "Set up a documentation set and write the first page"
type: tutorial
-->

# Getting Started with diataxis

In this tutorial we will set up a documentation set, write two pages of
different types, and read the map that shows where they sit.

## Initialize the set

Run this from the root of a project:

```console
$ diataxis init
diataxis: wrote [CWD]/diataxis.yml
diataxis: pages live in [CWD]/docs

```

The command writes `diataxis.yml` and makes a `docs` directory. The
output names both.

## Name the set

The map we will read at the end prints the set's title. Without one, it
prints the name of the project directory. Open `diataxis.yml` and add a
title:

```yaml {filename="diataxis.yml"}
docs_dir: docs
title: acme
```

## Write a tutorial

Every page declares a type. This one is a tutorial:

```console
$ diataxis page create "Getting started with acme" -T tutorial --description "Install acme and run the first job"
getting-started-with-acme (tutorial)

```

The command prints the page id and its type. The id comes from the
title. It is the file stem, and it becomes the page's published path, so
it carries no random prefix.

## Write a how-to guide

Now a page for a reader who is already competent:

```console
$ diataxis page create "How to rotate the signing key" -T how-to
how-to-rotate-the-signing-key (how-to)

```

Notice that the title starts with "How to". A how-to title states a
goal, so a reader scanning a list sees which page does the thing they
want. Try it without the words, then run the check:

```console
$ diataxis page create "Rotating the signing key" -T how-to
rotating-the-signing-key (how-to)

$ diataxis check
? 1
fault  rotating-the-signing-key     a how-to title states a goal: "Rotating the signing key" does not start with "How to"
advice  the set                      no reference pages
advice  the set                      no explanation pages
Error:   x 1 fault(s)


```

The check reports the title as a fault and exits with status 1. Remove
that page again:

```console
$ diataxis page delete rotating-the-signing-key
removed rotating-the-signing-key

```

## Read the map

The map prints the four types in Diátaxis order, with the pages under
each:

```console
$ diataxis map
acme

tutorial  informs action, serves acquisition of skill — 1 page(s)
  getting-started-with-acme    Getting started with acme

how-to  informs action, serves application of skill — 1 page(s)
  how-to-rotate-the-signing-key How to rotate the signing key

reference  informs cognition, serves application of skill — 0 page(s)
  (none)

explanation  informs cognition, serves acquisition of skill — 0 page(s)
  (none)

```

Two quadrants hold pages and two are empty. That is what the map is
for: a list of pages hides a gap, and a grid shows it.

## Check the set

Run the check again:

```console
$ diataxis check
advice  the set                      no reference pages
advice  the set                      no explanation pages

```

The two empty types come back as advice rather than faults, and the
check exits with status 0.

You have built a typed documentation set, and you can see what it is
missing.

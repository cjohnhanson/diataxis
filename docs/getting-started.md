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

```sh
diataxis init
```

The command writes `diataxis.yml` and makes a `docs` directory. The
output names both:

```
diataxis: wrote diataxis.yml
diataxis: pages live in docs
```

## Write a tutorial

Every page declares a type. This one is a tutorial:

```sh
diataxis page create "Getting started with acme" -T tutorial \
  --description "Install acme and run the first job"
```

The command prints the page id and its type:

```
getting-started-with-acme (tutorial)
```

The id comes from the title. It is the file stem, and it becomes the
page's published path, so it carries no random prefix.

## Write a how-to guide

Now a page for a reader who is already competent:

```sh
diataxis page create "How to rotate the signing key" -T how-to
```

Notice that the title starts with "How to". A how-to title states a
goal, so a reader scanning a list sees which page does the thing they
want. Try it without the words:

```sh
diataxis page create "Rotating the signing key" -T how-to
diataxis check
```

The check reports the title:

```
fault  rotating-the-signing-key     a how-to title states a goal: "Rotating the signing key" does not start with "How to"
```

Remove that page again:

```sh
diataxis page delete rotating-the-signing-key
```

## Read the map

```sh
diataxis map
```

The map prints the four types in Diátaxis order, with the pages under
each:

```
docs

tutorial  informs action, serves acquisition of skill — 1 page(s)
  getting-started-with-acme    Getting started with acme

how-to  informs action, serves application of skill — 1 page(s)
  how-to-rotate-the-signing-key  How to rotate the signing key

reference  informs cognition, serves application of skill — 0 page(s)
  (none)

explanation  informs cognition, serves acquisition of skill — 0 page(s)
  (none)
```

Two quadrants hold pages and two are empty. That is what the map is
for: a list of six pages hides a gap, and a grid shows it.

## Check the set

```sh
diataxis check
```

The two empty types come back as advice rather than faults:

```
advice  the set    no reference pages
advice  the set    no explanation pages
```

You have built a typed documentation set, and you can see what it is
missing.

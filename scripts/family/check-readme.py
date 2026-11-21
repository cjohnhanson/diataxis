#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = ["markdown-it-py==4.2.0"]
# ///
"""Check the shape every family README shares.

    uv run check-readme.py TOOL [README]

README defaults to README.md. The rules:

1. The README opens with a level-1 heading whose text names TOOL as a
   word (an emoji or code span around the name is fine).
2. No other level-1 heading follows.
3. A paragraph sits between that heading and the first section.
4. One section is `## Install` or `## Installation`.
5. The last section is `## License`.

Headings are read as CommonMark does: ATX and setext headings count;
headings inside fences, code blocks, and HTML comments do not. Exit
status 1 on any finding, with one `file:line: message` per finding; 2 on
a usage error or a file that cannot be read.
"""

import sys

from markdown_it import MarkdownIt


def heading_text(inline):
    """The heading's words, with code spans and emphasis unwrapped."""
    return "".join(
        child.content
        for child in (inline.children or [])
        if child.type in ("text", "code_inline")
    )


def check(tool, path):
    with open(path, encoding="utf-8") as handle:
        text = handle.read()
    tokens = MarkdownIt("commonmark").parse(text)
    headings = []
    first_block = None
    for position, token in enumerate(tokens):
        if (
            token.level == 0
            and token.map is not None
            and first_block is None
            and token.nesting != -1
        ):
            first_block = (token, position)
        if token.type == "heading_open":
            title = heading_text(tokens[position + 1]).strip()
            headings.append((token.map[0] + 1, int(token.tag[1]), title, position))
    findings = []
    opener = first_block[0] if first_block else None
    if not (opener and opener.type == "heading_open" and opener.tag == "h1"):
        findings.append(
            (
                1,
                f"the README does not open with a level-1 heading; start it with `# {tool}`",
            )
        )
    elif tool not in headings[0][2].split():
        findings.append(
            (headings[0][0], f"the title does not name {tool}; write `# {tool}`")
        )
    for line, level, _, _ in headings[1:]:
        if level == 1:
            findings.append((line, "a second level-1 heading; a README has one title"))
    sections = [
        (line, title, position)
        for line, level, title, position in headings
        if level == 2
    ]
    end = sections[0][2] if sections else len(tokens)
    start = headings[0][3] + 3 if headings and headings[0][1] == 1 else 0
    if not any(token.type == "paragraph_open" for token in tokens[start:end]):
        findings.append(
            (
                1,
                "no paragraph between the title and the first section; add one that says what the tool is",
            )
        )
    if not any(
        title.lower() in {"install", "installation"} for _, title, _ in sections
    ):
        findings.append((1, "no `## Install` or `## Installation` section"))
    if not sections or sections[-1][1].lower() != "license":
        findings.append(
            (sections[-1][0] if sections else 1, "the last section is not `## License`")
        )
    return [f"{path}:{line}: {message}" for line, message in findings]


def main(argv):
    if len(argv) not in (1, 2) or argv[0] in ("-h", "--help"):
        print("usage: check-readme.py TOOL [README]", file=sys.stderr)
        return 2
    path = argv[1] if len(argv) == 2 else "README.md"
    try:
        findings = check(argv[0], path)
    except OSError as error:
        print(f"{path}: cannot read: {error.strerror}", file=sys.stderr)
        return 2
    for finding in findings:
        print(finding)
    return 1 if findings else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

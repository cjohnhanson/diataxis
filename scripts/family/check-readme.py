#!/usr/bin/env python3
"""Check the shape every family README shares.

    check-readme.py TOOL [README]

README defaults to README.md. The rules:

1. The first line is a level-1 heading that names TOOL.
2. No other level-1 heading follows.
3. A paragraph sits between that heading and the first section.
4. One section is `## Install` or `## Installation`.
5. The last section is `## License`.

Headings inside fences do not count. Exit status 1 on any finding, with
one `file:line: message` per finding.
"""

import re
import sys

FENCE = re.compile(r"^ {0,3}(`{3,}|~{3,})")
HEADING = re.compile(r"^(#{1,6})\s+(.*?)\s*#*\s*$")


def headings(lines):
    fence = None
    for number, line in enumerate(lines, 1):
        match = FENCE.match(line)
        if fence is None and match:
            fence = match.group(1)
            continue
        if fence is not None:
            if (
                match
                and match.group(1)[0] == fence[0]
                and len(match.group(1)) >= len(fence)
                and not line.strip(" `~")
            ):
                fence = None
            continue
        heading = HEADING.match(line)
        if heading:
            yield number, len(heading.group(1)), heading.group(2)


def check(tool, path):
    with open(path, encoding="utf-8") as handle:
        lines = handle.read().splitlines()
    found = list(headings(lines))
    findings = []
    first = HEADING.match(lines[0]) if lines else None
    if not first or len(first.group(1)) != 1 or tool not in first.group(2).split():
        findings.append((1, f"the first line is a level-1 heading naming {tool}"))
    for number, level, _ in found[1:]:
        if level == 1:
            findings.append((number, "a README has one level-1 heading"))
    sections = [(number, title) for number, level, title in found if level == 2]
    intro_end = sections[0][0] if sections else len(lines) + 1
    if not any(
        line.strip() and not line.startswith("#") for line in lines[1 : intro_end - 1]
    ):
        findings.append((1, "a paragraph follows the title before the first section"))
    if not any(title.lower() in {"install", "installation"} for _, title in sections):
        findings.append(
            (1, "a README has an `## Install` or `## Installation` section")
        )
    if not sections or sections[-1][1].lower() != "license":
        findings.append(
            (sections[-1][0] if sections else 1, "the last section is `## License`")
        )
    return [f"{path}:{number}: {message}" for number, message in findings]


def main(argv):
    if len(argv) not in (1, 2):
        print(__doc__.strip().splitlines()[2].strip(), file=sys.stderr)
        return 2
    findings = check(argv[0], argv[1] if len(argv) == 2 else "README.md")
    for finding in findings:
        print(finding)
    return 1 if findings else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

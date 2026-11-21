#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = ["markdown-it-py==4.2.0"]
# ///
"""Fail a tool command that a docs test cannot run.

    uv run check-fences.py [--tool NAME]... FILE...

The family tools are the names in TOOLS below: the eng.rip tools and
gaff's short name, gaffr. `--tool` adds a name, so a tool that calls the
family workflow is checked whether or not TOOLS lists it.

Each tool repository runs its README and docs pages through trycmd in
`cargo test`. trycmd runs a block that opens with backticks at column 0
and whose info string is empty, `console`, or `trycmd`. A command
anywhere else is never run, so it can go wrong without a failing test.
These rules make the blocks trycmd runs and the blocks a reader sees as
examples the same set:

1. Every fence names a language.
2. trycmd runs exactly the ```console fences: backticks at column 0,
   outside lists and quotes, with no other words in the info string. A
   block trycmd runs that is anything else, or a `console` fence trycmd
   skips (`console,ignore`, a tilde or indented fence), fails.
3. A `console` block starts with a `$ ` command line.
4. A family tool command appears only in a `console` block, except under
   an `Install` or `Installation` section, whose commands change the
   reader's machine. A command is a line after a prompt (`$ `, `% `,
   `❯ `, `PS>`) in any fence, or a line in a shell or session fence, that
   names a family tool as a word or a path's last part; or, in a `text`
   fence or an indented code block, a line that starts with a tool name
   and is not a synopsis (`<placeholder>`, `[option]`) or a version line.
   Config languages such as YAML and TOML name tools without running them.
   A command that installs or locates a tool, a heredoc body, and a `#`
   comment in a shell fence pass. The section is the nearest level-2
   heading.

Exit status 1 on any finding, with one `file:line: message` per finding;
2 on a usage error or a file that cannot be read.
"""

import os
import re
import shlex
import sys

from markdown_it import MarkdownIt

TOOLS = {
    "tundish",
    "gaff",
    "gaffr",
    "tisket",
    "zettel",
    "almanac",
    "diataxis",
    "missouri",
}
SHELL_LANGS = {
    "sh",
    "bash",
    "shell",
    "zsh",
    "fish",
    "ksh",
    "powershell",
    "pwsh",
    "ps1",
    "bat",
    "cmd",
    "dockerfile",
    "just",
    "justfile",
    "make",
    "makefile",
    "csh",
    "tcsh",
    "nu",
    "nushell",
    "shell-session",
    "shellsession",
    "terminal",
}
PLAIN_LANGS = {"text", "txt", "plain", "plaintext", "output", "none"}
INSTALL_OR_LOCATE = re.compile(
    r"^\s*(?:RUN\s+)?(?:sudo\s+)?(?:"
    r"(?:pip3?|pipx|brew|apt(?:-get)?|dnf)\s+install"
    r"|cargo\s+(?:install|binstall)"
    r"|npm\s+(?:install|i)"
    r"|uv\s+(?:add|pip\s+install|tool\s+install)"
    r"|nix\s+profile\s+(?:install|add)"
    r"|command\s+-v|which|type"
    r")\b"
)
INSTALL_SECTIONS = {"install", "installation"}
PROMPT = re.compile(r"^\s*(?:\$|%|❯|PS>|PS [^>]*>)\s+")
# `<<<` is a here-string, not a heredoc.
HEREDOC = re.compile(r"(?<!<)<<(?!<)-?\s*['\"]?([A-Za-z_][A-Za-z0-9_]*)")
# Shell operators that start a new simple command.
OPERATORS = re.compile(r"&&|\|\||[;|&(){}`\n]|\$\(")
VERSION = re.compile(r"^v?\d+(?:\.\d+)*\S*$")


def trycmd_blocks(lines):
    """The blocks trycmd 1.2 runs, by its own rules (`parse_trycmd` in
    trycmd's schema.rs): (opening line index, info string, ignored)."""
    blocks = []
    marker = None
    for index, line in enumerate(lines):
        if marker is not None:
            if line.startswith(marker):
                marker = None
            continue
        ticks = len(line) - len(line.lstrip("`"))
        if ticks < 3:
            continue
        marker = line[:ticks]
        info = line[ticks:].strip()
        parts = info.split(",")
        if not info or parts[0] in ("console", "trycmd"):
            blocks.append((index, info, "ignore" in parts[1:]))
    return blocks


def words(text):
    try:
        return shlex.split(text, comments=True)
    except ValueError:
        return text.split()


def is_tool(word, first):
    """A word names a tool when it is the tool's name, or a path to it in
    command position (`./target/release/tundish`), so a directory such as
    `~/.config/gaff` passes."""
    if "://" in word or os.path.basename(word) not in TOOLS:
        return False
    return "/" not in word or first


def names_tool(line):
    """Whether a shell line runs a family tool in any of its commands."""
    for segment in OPERATORS.split(line):
        # A command that installs or locates a tool names it without
        # running it, and a reader runs it once on their own machine, so
        # no docs test can run it either.
        if INSTALL_OR_LOCATE.match(segment) or not segment.strip():
            continue
        for position, word in enumerate(words(segment)):
            # A quoted word such as `bash -c "tundish pull"` holds commands.
            if " " in word or "$(" in word or "`" in word:
                if names_tool(word):
                    return True
            elif is_tool(word, position == 0):
                return True
    return False


def invokes_tool(line, lang):
    """Whether a line in a non-console block of `lang` runs a family tool."""
    prompt = PROMPT.match(line)
    if prompt:
        return names_tool(line[prompt.end() :])
    if lang in SHELL_LANGS:
        return not line.lstrip().startswith("#") and names_tool(line)
    if lang not in PLAIN_LANGS:
        return False
    # A plain line names a command when it starts with a tool name, unless
    # it is a synopsis (`<placeholder>`, `[option]`) or a version line.
    parts = line.split()
    return (
        bool(parts)
        and is_tool(parts[0], True)
        and not re.search(r"<[^>]+>|\[[^\]]+\]", line)
        and not (len(parts) == 2 and VERSION.match(parts[1]))
    )


def code_findings(body, start, lang):
    """Tool commands in the body lines of a block that never runs. A
    shell heredoc's body is data, not commands."""
    found = []
    heredoc = None
    for offset, line in enumerate(body):
        if heredoc is not None:
            if line.strip() == heredoc:
                heredoc = None
            continue
        if lang in SHELL_LANGS and (match := HEREDOC.search(line)):
            heredoc = match.group(1)
        if invokes_tool(line, lang):
            found.append(start + offset)
    return found


def check(path):
    with open(path, encoding="utf-8") as handle:
        text = handle.read()
    # Split only on newlines, as markdown-it and trycmd do, so line
    # numbers agree across the three.
    lines = [line.rstrip("\r") for line in text.split("\n")]
    findings = []
    runs = {index: (info, ignored) for index, info, ignored in trycmd_blocks(lines)}
    seen = set()
    section = ""
    tokens = MarkdownIt("commonmark").parse(text)
    for position, token in enumerate(tokens):
        if token.type == "heading_open" and token.tag in ("h1", "h2"):
            title = tokens[position + 1].content.strip().lower()
            section = title if token.tag == "h2" else ""
            continue
        if token.type not in ("fence", "code_block") or token.map is None:
            continue
        first, last = token.map
        install = section in INSTALL_SECTIONS
        if token.type == "code_block":
            if not install:
                # An indented block holds commands as often as config or
                # output, so it is read as plain text.
                for index in code_findings(lines[first:last], first, "text"):
                    findings.append(
                        (
                            index,
                            "tool command in an indented code block; use a ```console fence",
                        )
                    )
            continue
        info = token.info.strip()
        head = info.split(",")[0].split()
        lang = head[0].lower() if head else ""
        opening = lines[first]
        top = token.level == 0 and opening.startswith("```")
        body = token.content.splitlines()
        closed = (
            last <= len(lines)
            and last - 1 > first
            and lines[last - 1].lstrip(" >").startswith(token.markup)
        )
        if not closed:
            findings.append((first, "fence is never closed"))
        if not lang:
            findings.append((first, "fence has no language; trycmd runs it as a test"))
        elif lang in ("console", "trycmd"):
            if first in runs and runs[first][1]:
                findings.append(
                    (
                        first,
                        f"`{info}` hides its commands from the docs test; use ```console",
                    )
                )
            elif info != "console":
                findings.append(
                    (
                        first,
                        f"trycmd does not run a `{info}` fence; write ```console with no other words",
                    )
                )
            elif token.level > 0:
                findings.append(
                    (
                        first,
                        "trycmd does not run a fence inside a list or quote; move it to the top level",
                    )
                )
            elif not top or first not in runs:
                findings.append(
                    (
                        first,
                        "trycmd does not run this fence; open it with ``` at column 0",
                    )
                )
            elif body and not body[0].startswith("$ "):
                findings.append(
                    (
                        first + 1,
                        "this `console` block does not start with a `$ ` command line",
                    )
                )
            seen.add(first)
        elif not install:
            for index in code_findings(body, first + 1, lang):
                findings.append(
                    (
                        index,
                        f"tool command in a `{lang}` fence; use `console` so the docs test runs it",
                    )
                )
        if top:
            seen.add(first)
    for index in runs:
        if index not in seen:
            findings.append(
                (
                    index,
                    "trycmd runs this block, but it is not a ```console fence a reader sees",
                )
            )
    return [f"{path}:{index + 1}: {message}" for index, message in sorted(findings)]


def main(argv):
    paths = []
    args = iter(argv)
    for arg in args:
        if arg == "--tool":
            name = next(args, "")
            if not name:
                print("--tool needs a name", file=sys.stderr)
                return 2
            TOOLS.add(name)
        elif arg in ("-h", "--help"):
            print(__doc__.strip())
            return 0
        else:
            paths.append(arg)
    if not paths:
        print("usage: check-fences.py [--tool NAME]... FILE...", file=sys.stderr)
        return 2
    findings = []
    for path in paths:
        try:
            findings.extend(check(path))
        except OSError as error:
            print(f"{path}: cannot read: {error.strerror}", file=sys.stderr)
            return 2
    for finding in findings:
        print(finding)
    return 1 if findings else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

"""Tests for the family docs checks.

Run: uv run --with markdown-it-py==4.2.0 python -m unittest discover scripts/family
"""

import importlib.util
import pathlib
import tempfile
import textwrap
import unittest

HERE = pathlib.Path(__file__).parent


def load(name):
    spec = importlib.util.spec_from_file_location(
        name.replace("-", "_"), HERE / f"{name}.py"
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


fences = load("check-fences")
readme = load("check-readme")


def write(text):
    handle = tempfile.NamedTemporaryFile(
        "w", suffix=".md", delete=False, encoding="utf-8"
    )
    handle.write(textwrap.dedent(text).lstrip())
    handle.close()
    return handle.name


class Fences(unittest.TestCase):
    def findings(self, text):
        return [finding.split(": ", 1)[1] for finding in fences.check(write(text))]

    def test_console_block_passes(self):
        self.assertEqual(
            self.findings("```console\n$ tundish --version\ntundish 0.1.2\n```\n"), []
        )

    def test_tool_in_sh_fence_fails(self):
        found = self.findings("## Usage\n\n```sh\ntundish pull inbox --as me\n```\n")
        self.assertEqual(len(found), 1)
        self.assertIn("use `console`", found[0])

    def test_tool_after_pipe_fails(self):
        self.assertEqual(
            len(self.findings("```bash\necho '{}' | tisket new x\n```\n")), 1
        )

    def test_tool_after_env_assignment_fails(self):
        self.assertEqual(len(self.findings("```sh\nFOO=1 zettel list\n```\n")), 1)

    def test_dollar_line_in_text_fence_fails(self):
        self.assertEqual(len(self.findings("```text\n$ gaff check\n```\n")), 1)

    def test_tool_name_in_text_fence_without_prompt_passes(self):
        self.assertEqual(self.findings("```text\ntundish <command>\n```\n"), [])

    def test_yaml_config_naming_a_tool_passes(self):
        self.assertEqual(
            self.findings("```yaml\ncommand: [tundish, pull, inbox]\n```\n"), []
        )

    def test_install_section_is_exempt(self):
        text = "## Install\n\n```sh\ncargo install tundish\n```\n\n### From source\n\n```sh\n./target/release/tundish --version\ntundish --version\n```\n"
        self.assertEqual(self.findings(text), [])

    def test_section_after_install_is_checked(self):
        text = "## Install\n\n```sh\ntundish --version\n```\n\n## Usage\n\n```sh\ntundish --version\n```\n"
        self.assertEqual(len(self.findings(text)), 1)

    def test_untagged_fence_fails(self):
        found = self.findings("```\nanything\n```\n")
        self.assertEqual(len(found), 1)
        self.assertIn("no language", found[0])

    def test_console_without_prompt_fails(self):
        self.assertEqual(len(self.findings("```console\ntundish 0.1.2\n```\n")), 1)

    def test_longer_outer_fence_holds_inner_fence(self):
        text = "````markdown\n```sh\ntundish pull\n```\n````\n"
        self.assertEqual(self.findings(text), [])

    def test_ignored_console_block_fails(self):
        for info in ("console,ignore", "trycmd,ignore"):
            found = self.findings(f"```{info}\n$ tundish pull\n```\n")
            self.assertTrue(any("hides" in f for f in found), (info, found))

    def test_console_with_extra_words_fails(self):
        for info in ("console ignore", 'console title="x"', "trycmd"):
            self.assertEqual(
                len(self.findings(f"```{info}\n$ tundish pull\n```\n")), 1, info
            )

    def test_tilde_console_fence_fails(self):
        self.assertEqual(len(self.findings("~~~console\n$ tundish pull\n~~~\n")), 1)

    def test_indented_console_fence_fails(self):
        text = "Run:\n\n  ```console\n$ tundish pull\n  ```\n"
        self.assertEqual(len(self.findings(text)), 1)

    def test_console_fence_in_a_list_fails(self):
        self.assertEqual(
            len(self.findings("- step\n\n  ```console\n  $ tundish pull\n  ```\n")), 1
        )

    def test_console_inside_a_tilde_fence_fails(self):
        # trycmd ignores tilde fences and runs the inner backtick block.
        found = self.findings("~~~markdown\n```console\n$ tundish pull\n```\n~~~\n")
        self.assertTrue(any("trycmd runs this block" in f for f in found), found)

    def test_command_shapes_in_shell_fences_fail(self):
        for line in (
            "./target/release/tundish pull",
            "env -i tundish pull",
            "sudo -E gaff check",
            "xargs tisket show",
            "nohup tundish pull",
            "if tundish check; then :; fi",
            "uvx zettel list",
            "npx missouri run",
            'bash -c "tundish pull"',
        ):
            self.assertEqual(len(self.findings(f"```sh\n{line}\n```\n")), 1, line)

    def test_other_prompts_fail(self):
        for line in ("% tundish pull", "❯ gaff check"):
            self.assertEqual(len(self.findings(f"```text\n{line}\n```\n")), 1, line)

    def test_command_in_a_text_fence_fails(self):
        self.assertEqual(
            len(self.findings("```text\ntundish pull inbox --as me\n```\n")), 1
        )

    def test_indented_code_block_fails(self):
        self.assertEqual(len(self.findings("Run:\n\n    tundish pull inbox\n")), 1)

    def test_install_and_locate_commands_pass_anywhere(self):
        for line in (
            "pip install tundish",
            "uv add tundish",
            "cargo install gaff",
            "command -v tundish",
            "which zettel",
            "npm install -g tisket",
            "brew install cjohnhanson/tap/almanac",
        ):
            self.assertEqual(
                self.findings(f"## Usage\n\n```sh\n{line}\n```\n"), [], line
            )

    def test_chained_and_substituted_commands_fail(self):
        for line in (
            "cargo install tundish && tundish pull",
            "type tundish && tundish pull",
            "which gaff; gaff check",
            'echo "$(tundish --version)"',
            "v=$(tisket new foo)",
            "echo `gaff check`",
            "cd x&&tundish pull",
        ):
            self.assertEqual(len(self.findings(f"```sh\n{line}\n```\n")), 1, line)

    def test_here_string_does_not_hide_later_lines(self):
        self.assertEqual(
            len(self.findings('```sh\ngrep x <<< "hello"\ntundish pull\n```\n')), 1
        )

    def test_language_case_is_ignored(self):
        for info in ("Bash", "Text"):
            self.assertEqual(
                len(self.findings(f"```{info}\ntundish pull inbox\n```\n")), 1, info
            )
        self.assertEqual(len(self.findings("```Console\n$ tundish pull\n```\n")), 1)

    def test_config_paths_and_versions_pass(self):
        self.assertEqual(self.findings("```sh\nmkdir -p ~/.config/gaff\n```\n"), [])
        self.assertEqual(self.findings("```text\ntundish 0.1.2\n```\n"), [])
        self.assertEqual(
            self.findings("```dockerfile\nRUN cargo install tundish\n```\n"), []
        )

    def test_indented_yaml_naming_a_tool_passes(self):
        self.assertEqual(
            self.findings("List:\n\n    - name: gaff\n      path: skills/gaff\n"), []
        )

    def test_fence_in_a_blockquote_closes(self):
        self.assertEqual(self.findings("> ```yaml\n> a: 1\n> ```\n"), [])

    def test_form_feed_keeps_line_numbers(self):
        self.assertEqual(self.findings("a\x0cb\n\n```yaml\nx: 1\n```\n"), [])

    def test_heredoc_body_naming_a_tool_passes(self):
        self.assertEqual(
            self.findings("```sh\ncat <<EOF\ntundish is great\nEOF\n```\n"), []
        )

    def test_url_naming_a_tool_passes(self):
        self.assertEqual(
            self.findings("```sh\ngit clone https://github.com/x/tundish\n```\n"), []
        )

    def test_html_comment_holding_a_fence_passes(self):
        self.assertEqual(self.findings("<!--\n```sh\ntundish pull\n```\n-->\n"), [])

    def test_shell_comment_naming_a_tool_passes(self):
        self.assertEqual(
            self.findings("```sh\n# tundish is installed\ncargo build\n```\n"), []
        )

    def test_unclosed_fence_fails(self):
        self.assertEqual(len(self.findings("```sh\necho\n")), 1)


class Readme(unittest.TestCase):
    GOOD = "# tundish\n\nA local inbox.\n\n## Installation\n\nInstall it.\n\n## License\n\nMIT.\n"

    def findings(self, text, tool="tundish"):
        return [
            finding.split(": ", 1)[1] for finding in readme.check(tool, write(text))
        ]

    def test_good_readme_passes(self):
        self.assertEqual(self.findings(self.GOOD), [])

    def test_install_heading_passes(self):
        self.assertEqual(
            self.findings(self.GOOD.replace("## Installation", "## Install")), []
        )

    def test_wrong_title_fails(self):
        self.assertEqual(len(self.findings(self.GOOD, tool="gaff")), 1)

    def test_emoji_before_name_passes(self):
        self.assertEqual(
            self.findings(self.GOOD.replace("# tundish", "# 🪝 tundish")), []
        )

    def test_missing_install_fails(self):
        self.assertEqual(
            len(self.findings(self.GOOD.replace("## Installation", "## Setup"))), 1
        )

    def test_license_not_last_fails(self):
        self.assertEqual(len(self.findings(self.GOOD + "\n## Related\n\nx\n")), 1)

    def test_second_h1_fails(self):
        self.assertEqual(
            len(self.findings(self.GOOD.replace("Install it.", "# Other"))), 1
        )

    def test_h1_inside_fence_passes(self):
        self.assertEqual(
            self.findings(
                self.GOOD.replace("Install it.", "```markdown\n# Other\n```")
            ),
            [],
        )

    def test_missing_intro_fails(self):
        self.assertEqual(len(self.findings(self.GOOD.replace("A local inbox.", ""))), 1)

    def test_setext_second_h1_fails(self):
        text = self.GOOD.replace("Install it.", "Another\n=======")
        self.assertEqual(len(self.findings(text)), 1)

    def test_h1_in_an_html_comment_passes(self):
        text = self.GOOD.replace("Install it.", "<!--\n# draft\n-->")
        self.assertEqual(self.findings(text), [])

    def test_setext_license_passes(self):
        self.assertEqual(
            self.findings(self.GOOD.replace("## License", "License\n-------")), []
        )

    def test_indented_section_after_license_fails(self):
        self.assertEqual(len(self.findings(self.GOOD + "\n ## Usage\n\nx\n")), 1)

    def test_code_span_title_passes(self):
        self.assertEqual(
            self.findings(self.GOOD.replace("# tundish", "# `tundish`")), []
        )

    def test_unreadable_file_exits_2(self):
        self.assertEqual(readme.main(["tundish", "/nonexistent/README.md"]), 2)


class FencesCli(unittest.TestCase):
    def test_tool_option_adds_a_name(self):
        path = write("```sh\nringer sync --all\n```\n")
        self.assertEqual(fences.main([path]), 0)
        self.assertEqual(fences.main(["--tool", "ringer", path]), 1)
        fences.TOOLS.discard("ringer")

    def test_comma_info_string_does_not_crash(self):
        self.assertEqual(fences.main([write("``` ,ignore\nx\n```\n")]), 1)

    def test_no_files_and_missing_files_exit_2(self):
        self.assertEqual(fences.main([]), 2)
        self.assertEqual(fences.main(["/nonexistent.md"]), 2)


if __name__ == "__main__":
    unittest.main()

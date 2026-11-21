//! The docs are tests.
//!
//! `console_examples` runs every `console` block in README.md and
//! docs/*.md against the built binary with trycmd, and compares the
//! printed output with the block. Each page runs top to bottom in its own
//! scratch directory, with `HOME` and the XDG directories in that
//! directory, so no page reads the machine's root set or store cache.
//! diataxis reads its user config from the passwd home directory, not
//! `$HOME`, so the `diataxis` wrapper also passes the hidden
//! `--user-config` flag with a path under the page's `HOME`.
//!
//! Every command a page runs starts through a small `/bin/sh` wrapper that
//! clears the environment (`env -i`) and sets only the variables below.
//! Without it an inherited variable changes the output: `MDSTORE_REGISTRY`
//! or `XDG_CACHE_HOME` points diataxis at the machine's registry or store
//! cache.
//!
//! No page reaches the network. A page that composes sets declares a
//! `path:` set that its setup builds beside the working directory.
//!
//! trycmd writes the working directory as `[CWD]` in output, and the
//! directory that holds it as `[PARENT]`.
//!
//! A fence with `{filename="<path>"}` in its info string is a file the
//! reader writes. Its body is written to that path in the page's working
//! directory when the page reaches it, so the commands after it see the
//! file.
//!
//! A page can have fixtures in tests/docs/<page>/: `setup.md` holds
//! `console` blocks (and file fences) that run before the page, for
//! examples that assume earlier state; `work/` is copied into the working
//! directory and `home/` into `HOME`.
//!
//! `help_matches_snapshots` compares the `--help` output of every visible
//! command with tests/snapshots/help/. docs/cli-reference.md is generated
//! from the same definitions by tests/cli_reference.rs. The Rust examples
//! in README.md and the adoption guide compile as doctests
//! (`ReadmeDoctests` and `AdoptionGuideDoctests` in src/lib.rs).
//!
//! Write the real output into the pages and snapshots, then read the
//! diff: `TRYCMD=overwrite cargo test --test docs` for the examples,
//! `SNAPSHOTS=overwrite cargo test --test docs` for the help snapshots.

use std::path::{Path, PathBuf};
use std::process::Command;

use clap::CommandFactory as _;

const BIN: &str = env!("CARGO_BIN_EXE_diataxis");
/// clap wraps help to `COLUMNS` when no terminal is attached, so the
/// width is fixed for every run.
const COLUMNS: &str = "100";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// README.md and every page in docs/, in name order.
fn pages() -> Vec<PathBuf> {
    let mut pages: Vec<PathBuf> = std::fs::read_dir(root().join("docs"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .collect();
    pages.sort();
    pages.insert(0, root().join("README.md"));
    pages
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn scratch() -> (tempfile::TempDir, PathBuf) {
    let guard = tempfile::tempdir().unwrap();
    // Canonical, so a path diataxis prints resolved matches the path
    // trycmd writes as `[CWD]`.
    let path = guard.path().canonicalize().unwrap();
    (guard, path)
}

#[test]
fn console_examples() {
    let (_guard, scratch) = scratch();
    run_pages(&scratch, &pages());
}

/// Run every page, then fail naming each page whose examples differ.
fn run_pages(scratch: &Path, pages: &[PathBuf]) {
    let mut failed = Vec::new();
    for page in pages {
        if std::panic::catch_unwind(|| run_page(scratch, page)).is_err() {
            let name = page.strip_prefix(root()).unwrap_or(page);
            failed.push(name.display().to_string());
        }
    }
    assert!(
        failed.is_empty(),
        "examples differ from the binary in: {failed:?}; \
         `TRYCMD=overwrite cargo test --test docs` writes the real output, then read the diff"
    );
}

/// Run one page's `console` blocks in `scratch/<stem>/`.
fn run_page(scratch: &Path, page: &Path) {
    let stem = page.file_stem().unwrap().to_string_lossy().into_owned();
    let fixtures = root().join("tests/docs").join(&stem);
    let dir = scratch.join(&stem);
    // trycmd runs `page.md` in `page.in/`.
    let work = dir.join("page.in");
    let home = dir.join("home");
    let bin = dir.join("bin");
    for (from, to) in [
        (fixtures.join("work"), &work),
        (fixtures.join("home"), &home),
    ] {
        if from.is_dir() {
            copy_dir(&from, to);
        } else {
            std::fs::create_dir_all(to).unwrap();
        }
    }
    std::fs::create_dir_all(&bin).unwrap();
    // No setup.md means no setup; any other read error fails the page,
    // rather than running it without the state it assumes.
    let setup = match std::fs::read_to_string(fixtures.join("setup.md")) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => panic!("read the setup for {}: {e}", page.display()),
    };
    let body = std::fs::read_to_string(page).unwrap();
    let text = format!("{setup}{body}");

    // trycmd skips, and does not fail, a command it cannot resolve, and it
    // resolves an unregistered crate binary to the binary itself. So every
    // command a page runs is registered here, through a wrapper, from the
    // blocks trycmd runs, or the page fails.
    let mut registered = Vec::new();
    for name in commands(page, &text) {
        let wrapper = if name == "diataxis" {
            write_diataxis_wrapper(&bin)
        } else {
            let found = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
                .map(|dir| dir.join(&name))
                .find(|p| p.is_file());
            let path = found.unwrap_or_else(|| {
                panic!("{} runs `{name}`, which is not on PATH", page.display())
            });
            write_wrapper(&bin, &name, &path)
        };
        registered.push((name, wrapper));
    }

    let case = dir.join("page.md");
    let overwrite = std::env::var_os("TRYCMD").is_some_and(|m| m == "overwrite");
    let mut written = String::new();
    for (index, segment) in segments(page, &text).into_iter().enumerate() {
        if segment.text.lines().any(|l| l.starts_with("$ ")) {
            std::fs::write(&case, &segment.text).unwrap();
            let cases = trycmd::TestCases::new();
            for (name, wrapper) in &registered {
                cases.register_bin(name.clone(), wrapper);
            }
            let home_var = |sub: &str| home.join(sub).display().to_string();
            // A set beside the working directory prints as an absolute
            // path. `[PARENT]` stands for the directory that holds the
            // working directory, as `[CWD]` stands for the working
            // directory; the longer value is replaced first.
            cases
                .insert_var("[PARENT]", dir.display().to_string())
                .unwrap();
            cases
                .env("HOME", home.display().to_string())
                .env("XDG_CONFIG_HOME", home_var(".config"))
                .env("XDG_STATE_HOME", home_var(".local/state"))
                .env("XDG_DATA_HOME", home_var(".local/share"))
                .env("XDG_CACHE_HOME", home_var(".cache"))
                .case(&case)
                .run();
            if overwrite {
                written.push_str(&std::fs::read_to_string(&case).unwrap());
            } else {
                written.push_str(&segment.text);
            }
        } else {
            written.push_str(&segment.text);
        }
        if let Some((name, content)) = segment.file {
            assert!(
                !name.starts_with('/') && !name.split('/').any(|p| p == ".."),
                "{}: segment {index} writes {name}, outside the working directory",
                page.display()
            );
            let target = work.join(&name);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::write(target, content).unwrap();
        }
    }

    // TRYCMD=overwrite rewrote the scratch copies. Carry the page's part
    // back to the page; the setup part stays in tests/docs.
    if overwrite {
        let body = written.strip_prefix(&setup).unwrap_or_else(|| {
            panic!(
                "the setup for {} changed; run it again after updating setup.md",
                page.display()
            )
        });
        std::fs::write(page, body).unwrap();
    }
}

/// A run of a page that trycmd runs as one case, and the file the reader
/// writes at its end, if any.
struct Segment {
    text: String,
    file: Option<(String, String)>,
}

/// The value of `filename="..."` in a fence's info string.
fn filename(info: &str) -> Option<String> {
    let start = info.find("filename=\"")? + "filename=\"".len();
    let end = info[start..].find('"')?;
    Some(info[start..start + end].to_string())
}

/// Split a page after each fence that names a file, so the file can be
/// written before the commands that follow it run. Fences are read by the
/// same rules as `run_lines`.
fn segments(page: &Path, text: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut current = String::new();
    // The open fence: its backticks, the file it names, and its body.
    let mut fence: Option<(String, Option<String>, String)> = None;
    for line in text.split_inclusive('\n') {
        current.push_str(line);
        let bare = line.trim_end_matches(['\n', '\r']);
        if let Some((marker, file, body)) = fence.as_mut() {
            if bare.starts_with(marker.as_str()) {
                let file = file.take();
                let content = std::mem::take(body);
                fence = None;
                if let Some(name) = file {
                    segments.push(Segment {
                        text: std::mem::take(&mut current),
                        file: Some((name, content)),
                    });
                }
            } else {
                body.push_str(line);
            }
            continue;
        }
        let ticks = bare.len() - bare.trim_start_matches('`').len();
        if ticks < 3 {
            continue;
        }
        let info = &bare[ticks..];
        let file = filename(info);
        assert!(
            file.is_none() || !info.trim_start().starts_with('{'),
            "{}: a file fence needs a language before {{filename=...}}",
            page.display()
        );
        fence = Some((bare[..ticks].to_string(), file, String::new()));
    }
    segments.push(Segment {
        text: current,
        file: None,
    });
    segments
}

/// The lines of every block trycmd runs, read by trycmd 1.2's own rules
/// (`parse_trycmd` in its schema.rs): a fence with no info string, or
/// one whose language is `console` or `trycmd`, runs; the block ends at
/// the first line that starts with the opening backticks.
///
/// Panics on a `console,ignore` or `trycmd,ignore` block: trycmd skips
/// it, so its commands would never be checked.
fn run_lines<'a>(page: &Path, text: &'a str) -> Vec<&'a str> {
    let mut lines = Vec::new();
    // The open fence: its backticks, and whether trycmd runs its body.
    let mut fence: Option<(&str, bool)> = None;
    for line in text.lines() {
        if let Some((marker, runs)) = fence {
            if line.starts_with(marker) {
                fence = None;
            } else if runs {
                lines.push(line);
            }
            continue;
        }
        let ticks = line.len() - line.trim_start_matches('`').len();
        if ticks < 3 {
            continue;
        }
        let info = line[ticks..].trim();
        let mut parts = info.split(',');
        let lang = parts.next().unwrap_or_default();
        let runs = info.is_empty() || lang == "console" || lang == "trycmd";
        assert!(
            !(runs && parts.any(|p| p == "ignore")),
            "{}: `{info}` hides its commands from the docs test",
            page.display()
        );
        fence = Some((&line[..ticks], runs));
    }
    lines
}

/// The command word of every `$ ` line in the blocks trycmd runs.
///
/// Panics on an output line holding `\`: trycmd reads `\` in real output
/// as `/`, so such a line can never match the binary.
fn commands(page: &Path, text: &str) -> std::collections::BTreeSet<String> {
    let mut names = std::collections::BTreeSet::new();
    for line in run_lines(page, text) {
        if let Some(cmd) = line.strip_prefix("$ ") {
            // Split with the call trycmd 1.2 makes (`Shlex::new(raw.trim())`
            // in its schema.rs), which keeps the words before an unclosed
            // quote, so the command word is the program trycmd runs.
            if let Some(word) = shlex::Shlex::new(cmd.trim()).find(|w| !w.contains('=')) {
                names.insert(word);
            }
        } else if !line.starts_with("> ") {
            // `> ` continues a command; every other line is output.
            assert!(
                !line.contains('\\'),
                "{}: trycmd cannot match output holding a backslash; choose an example without one: {line}",
                page.display()
            );
        }
    }
    names
}

/// The variables every page command sees, and nothing else. `$HOME` and
/// the XDG directories are the page's, set by the test on the wrapper.
fn clean_env() -> String {
    format!(
        "env -i HOME=\"$HOME\" XDG_CONFIG_HOME=\"$XDG_CONFIG_HOME\" \
         XDG_STATE_HOME=\"$XDG_STATE_HOME\" XDG_DATA_HOME=\"$XDG_DATA_HOME\" \
         XDG_CACHE_HOME=\"$XDG_CACHE_HOME\" PATH=\"$PATH\" NO_COLOR=1 TERM=dumb \
         COLUMNS={COLUMNS}"
    )
}

fn quoted(path: &Path) -> String {
    // The wrappers quote paths in single quotes.
    assert!(
        !path.to_string_lossy().contains('\''),
        "cannot wrap a path holding a single quote: {}",
        path.display()
    );
    format!("'{}'", path.display())
}

fn write_script(path: &Path, script: &str) -> PathBuf {
    std::fs::write(path, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    path.to_path_buf()
}

/// `name` in the examples: `target` with the clean environment.
fn write_wrapper(bin: &Path, name: &str, target: &Path) -> PathBuf {
    write_script(
        &bin.join(name),
        &format!(
            "#!/bin/sh\nexec {} {} \"$@\"\n",
            clean_env(),
            quoted(target)
        ),
    )
}

/// `diataxis` in the examples: the built binary with the clean
/// environment, and `--user-config` under the page's `HOME`, because
/// diataxis reads its user config from the passwd home directory. The
/// flag is global and its last occurrence wins, so a page can still pass
/// its own.
fn write_diataxis_wrapper(bin: &Path) -> PathBuf {
    write_script(
        &bin.join("diataxis"),
        &format!(
            "#!/bin/sh\nexec {} {} --user-config \"$HOME/.config/diataxis/config.yml\" \"$@\"\n",
            clean_env(),
            quoted(Path::new(BIN))
        ),
    )
}

/// Every visible command path, depth first: `[]`, `["init"]`, ...
fn command_paths(cmd: &clap::Command, prefix: &[String], out: &mut Vec<Vec<String>>) {
    out.push(prefix.to_vec());
    for sub in cmd.get_subcommands() {
        if sub.is_hide_set() || sub.get_name() == "help" {
            continue;
        }
        let mut path = prefix.to_vec();
        path.push(sub.get_name().to_string());
        command_paths(sub, &path, out);
    }
}

fn visible_command_paths() -> Vec<Vec<String>> {
    let mut paths = Vec::new();
    command_paths(&diataxis::cli::Args::command(), &[], &mut paths);
    paths
}

fn help(path: &[String]) -> String {
    let out = Command::new(BIN)
        .args(path)
        .arg("--help")
        .env_clear()
        .env("NO_COLOR", "1")
        .env("TERM", "dumb")
        .env("COLUMNS", COLUMNS)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{path:?} --help failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

fn snapshot_dir() -> PathBuf {
    root().join("tests/snapshots/help")
}

/// The snapshot file of one command path: `diataxis-page-create.txt`.
fn snapshot_name(path: &[String]) -> String {
    let mut name = String::from("diataxis");
    for part in path {
        name.push('-');
        name.push_str(part);
    }
    name + ".txt"
}

/// Every visible command's `--help` output matches its snapshot.
#[test]
fn help_matches_snapshots() {
    let dir = snapshot_dir();
    std::fs::create_dir_all(&dir).unwrap();
    for path in visible_command_paths() {
        let file = dir.join(snapshot_name(&path));
        snapbox::Assert::new()
            .action_env("SNAPSHOTS")
            .eq(help(&path), snapbox::Data::read_from(&file, None).raw());
    }
}

/// Every snapshot names a visible command, so a removed or renamed
/// command leaves no stale snapshot behind.
#[test]
fn every_help_snapshot_names_a_visible_command() {
    let wanted: std::collections::BTreeSet<String> = visible_command_paths()
        .iter()
        .map(|p| snapshot_name(p))
        .collect();
    let mut have = std::collections::BTreeSet::new();
    for entry in std::fs::read_dir(snapshot_dir()).unwrap() {
        have.insert(entry.unwrap().file_name().to_string_lossy().into_owned());
    }
    assert_eq!(
        have, wanted,
        "tests/snapshots/help/ must hold one file per visible command: delete a stale file, \
         and write a missing one with `SNAPSHOTS=overwrite cargo test --test docs`"
    );
}

/// The body of every `toml` fence in a page. A fence closes only at a
/// line starting with its own opening backticks, as in `run_lines`. A
/// `~~~` fence panics: the docs use backtick fences only, and one that
/// this reader skipped would escape the check.
fn toml_fences(name: &str, page: &str) -> Vec<String> {
    let mut fences = Vec::new();
    // The open fence: its backticks, whether it is toml, and its body.
    let mut open: Option<(&str, bool, String)> = None;
    for line in page.lines() {
        if let Some((marker, is_toml, body)) = open.as_mut() {
            if line.starts_with(*marker) {
                if *is_toml {
                    fences.push(std::mem::take(body));
                }
                open = None;
            } else {
                body.push_str(line);
                body.push('\n');
            }
            continue;
        }
        assert!(
            !line.starts_with("~~~"),
            "{name}: a `~~~` fence: use backticks\n{line}"
        );
        let ticks = line.len() - line.trim_start_matches('`').len();
        if ticks >= 3 {
            let is_toml = line[ticks..].trim_start().starts_with("toml");
            open = Some((&line[..ticks], is_toml, String::new()));
        }
    }
    fences
}

/// A toml fence closes at its own backticks, so a shorter fence inside a
/// longer one is part of its body, and a toml fence inside a longer
/// non-toml fence is not read.
#[test]
fn a_toml_fence_closes_at_its_own_backticks() {
    let page = "````toml\n[dependencies]\n```\n````\n\n````markdown\n```toml\nx = 1\n```\n````\n";
    assert_eq!(toml_fences("p.md", page), ["[dependencies]\n```\n"]);
}

/// A `~~~` fence is refused, naming the page, since this reader would
/// otherwise skip a dependency example inside one.
#[test]
#[should_panic(expected = "p.md: a `~~~` fence: use backticks")]
fn a_tilde_fence_is_refused() {
    toml_fences("p.md", "~~~toml\n[dependencies]\n~~~\n");
}

/// Every key named `diataxis` in a TOML fence, at any depth, with its
/// dotted path: inline, as its own table, under a target table, or as a
/// dotted key. Each one is checked, so no form or table escapes. A fence
/// that does not parse panics.
fn diataxis_dependencies(page: &str, fence: &str) -> Vec<(String, toml_edit::Item)> {
    fn walk(
        table: &dyn toml_edit::TableLike,
        path: &str,
        out: &mut Vec<(String, toml_edit::Item)>,
    ) {
        for (key, item) in table.iter() {
            let here = if path.is_empty() {
                key.to_string()
            } else {
                format!("{path}.{key}")
            };
            if key == "diataxis" {
                out.push((here, item.clone()));
            } else if let Some(inner) = item.as_table_like() {
                walk(inner, &here, out);
            }
        }
    }
    let doc: toml_edit::DocumentMut = fence
        .parse()
        .unwrap_or_else(|e| panic!("{page}: a toml fence does not parse: {e}\n{fence}"));
    let mut found = Vec::new();
    walk(doc.as_table(), "", &mut found);
    found
}

/// The dependency examples the README and the adoption guide show name
/// this crate's version, turn the default features off, and name features
/// that exist. No command prints them, and cargo reads a feature name
/// that does not exist as an error only in a consumer's build.
#[test]
fn the_dependency_examples_name_this_version_and_real_features() {
    let manifest: toml_edit::DocumentMut = std::fs::read_to_string(root().join("Cargo.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let features = manifest["features"].as_table_like().unwrap();
    let version = env!("CARGO_PKG_VERSION");
    let minor = version.rsplit_once('.').unwrap().0;
    let mut seen = 0;
    for page in [
        "README.md",
        "docs/how-to-adopt-diataxis-in-an-existing-project.md",
    ] {
        let text = std::fs::read_to_string(root().join(page)).unwrap();
        for fence in toml_fences(page, &text) {
            for (table, item) in diataxis_dependencies(page, &fence) {
                seen += 1;
                let dep = item
                    .as_table_like()
                    .unwrap_or_else(|| panic!("{page}: `{table}` is not a table"));
                assert_eq!(
                    dep.get("version").and_then(|v| v.as_str()),
                    Some(minor),
                    "{page}: `{table}` must name version \"{minor}\""
                );
                assert_eq!(
                    dep.get("default-features").and_then(|v| v.as_bool()),
                    Some(false),
                    "{page}: `{table}` must set default-features = false, \
                     or a consumer compiles the whole CLI"
                );
                let named = dep
                    .get("features")
                    .and_then(|v| v.as_array())
                    .unwrap_or_else(|| panic!("{page}: `{table}` names no features"));
                for feature in named {
                    let feature = feature.as_str().unwrap();
                    assert!(
                        features.contains_key(feature),
                        "{page}: `{table}` names feature {feature}, which Cargo.toml lacks"
                    );
                }
            }
        }
    }
    assert!(seen >= 5, "found {seen} diataxis dependencies");
}

/// Every form cargo accepts is found, whatever table holds it: inline,
/// its own table (with a comment on the header), a target table, a dotted
/// key, and a quoted key.
#[test]
fn every_form_of_a_diataxis_dependency_is_found() {
    let fence = "[dependencies]\n\"diataxis\" = { version = \"0.2\" }\n\n\
                 [build-dependencies.diataxis] # the build helper\nversion = \"0.2\"\n\n\
                 [target.'cfg(unix)'.dependencies]\ndiataxis = \"0.1\"\n\n\
                 [dev-dependencies]\ndiataxis.version = \"0.2\"\n";
    let paths: Vec<String> = diataxis_dependencies("p.md", fence)
        .into_iter()
        .map(|(path, _)| path)
        .collect();
    assert_eq!(
        paths,
        [
            "dependencies.diataxis",
            "build-dependencies.diataxis",
            "target.cfg(unix).dependencies.diataxis",
            "dev-dependencies.diataxis",
        ]
    );
}

/// One wrong page among right ones fails the run, and the failure names
/// that page alone.
#[test]
fn a_wrong_page_fails_the_run_and_is_named() {
    if std::env::var_os("TRYCMD").is_some() {
        return;
    }
    let (_guard, scratch) = scratch();
    let right = scratch.join("right.md");
    std::fs::write(
        &right,
        "```console\n$ diataxis page list\n? 1\nError:   x invalid store configuration[..]\n...\n```\n",
    )
    .unwrap();
    let wrong = scratch.join("wrong.md");
    std::fs::write(
        &wrong,
        "```console\n$ diataxis compass\nOne question\n\n```\n",
    )
    .unwrap();
    let pages = [right, wrong.clone()];
    let err = std::panic::catch_unwind(|| run_pages(&scratch, &pages)).unwrap_err();
    let message = err.downcast_ref::<String>().cloned().unwrap_or_default();
    assert!(
        message.contains(&format!("[{:?}]", wrong.display().to_string())),
        "{message}"
    );
}

/// The harness itself: a page whose output differs from the binary fails,
/// and a page that matches passes.
#[test]
fn a_wrong_example_fails_and_a_right_one_passes() {
    if std::env::var_os("TRYCMD").is_some() {
        return; // overwrite mode would rewrite the wrong page and pass
    }
    let (_guard, scratch) = scratch();
    let right = scratch.join("right.md");
    std::fs::write(
        &right,
        "```console\n\
         $ diataxis init\n\
         diataxis: wrote [CWD]/diataxis.yml\n\
         diataxis: pages live in [CWD]/docs\n\
         \n\
         $ diataxis page create \"How to rotate a key\" -T how-to\n\
         how-to-rotate-a-key (how-to)\n\
         \n\
         ```\n",
    )
    .unwrap();
    run_page(&scratch, &right);
    let wrong = scratch.join("wrong.md");
    std::fs::write(&wrong, "```console\n$ diataxis init\nInitialized\n\n```\n").unwrap();
    assert!(std::panic::catch_unwind(|| run_page(&scratch, &wrong)).is_err());
}

/// The wrapper passes the binary's exit status through: a failing
/// command passes with `? 1` and fails the page without it.
#[test]
fn a_failing_command_needs_its_exit_status() {
    if std::env::var_os("TRYCMD").is_some() {
        return;
    }
    let (_guard, scratch) = scratch();
    let init = "$ diataxis init\n\
                diataxis: wrote [CWD]/diataxis.yml\n\
                diataxis: pages live in [CWD]/docs\n\
                \n";
    let marked = scratch.join("marked.md");
    std::fs::write(
        &marked,
        format!(
            "```console\n{init}$ diataxis page show nope\n? 1\nError:   x no page matches nope\n\n\n```\n"
        ),
    )
    .unwrap();
    let unmarked = scratch.join("unmarked.md");
    std::fs::write(
        &unmarked,
        format!(
            "```console\n{init}$ diataxis page show nope\nError:   x no page matches nope\n\n\n```\n"
        ),
    )
    .unwrap();
    run_page(&scratch, &marked);
    assert!(std::panic::catch_unwind(|| run_page(&scratch, &unmarked)).is_err());
}

/// A file fence is on disk for the commands after it, and a wrong output
/// after it fails the page.
#[test]
fn a_file_fence_lands_before_the_commands_after_it() {
    if std::env::var_os("TRYCMD").is_some() {
        return;
    }
    let (_guard, scratch) = scratch();
    let right = scratch.join("after.md");
    std::fs::write(
        &right,
        "```yaml {filename=\"diataxis.yml\"}\ndocs_dir: pages\ntitle: acme\n```\n\n\
         ```console\n$ diataxis map\nacme\n...\n```\n",
    )
    .unwrap();
    std::fs::create_dir_all(scratch.join("after/page.in/pages")).unwrap();
    run_page(&scratch, &right);
    let wrong = scratch.join("after-wrong.md");
    std::fs::write(
        &wrong,
        "```text {filename=\"a.txt\"}\nhello\n```\n\n```console\n$ cat a.txt\ngoodbye\n\n```\n",
    )
    .unwrap();
    assert!(std::panic::catch_unwind(|| run_page(&scratch, &wrong)).is_err());
}

/// A set beside the working directory, named by a relative `--root`,
/// prints under `[CWD]`, because diataxis joins the flag to the working
/// directory without resolving `..`. `[PARENT]` matches the directory
/// that holds the working directory, as `HOME` shows.
#[test]
fn paths_beside_the_working_directory_match_cwd_and_parent() {
    if std::env::var_os("TRYCMD").is_some() {
        return;
    }
    let (_guard, scratch) = scratch();
    let page = scratch.join("beside.md");
    std::fs::write(
        &page,
        "```console\n\
         $ mkdir ../team\n\
         \n\
         $ diataxis --root ../team init\n\
         diataxis: wrote [CWD]/../team/diataxis.yml\n\
         diataxis: pages live in [CWD]/../team/docs\n\
         \n\
         $ pwd\n\
         [CWD]\n\
         \n\
         $ printenv HOME\n\
         [PARENT]/home\n\
         \n\
         ```\n",
    )
    .unwrap();
    run_page(&scratch, &page);
}

/// A page reads the user config under its own `HOME`, never the one in
/// the passwd home directory. The fixtures in
/// tests/docs/user-config-fixture/ set a `root_store` that no machine
/// has, so the page fails on every machine when the wrapper stops
/// passing `--user-config`. They also prove that `home/` and `work/` are
/// copied.
#[test]
fn a_page_reads_no_machine_user_config() {
    if std::env::var_os("TRYCMD").is_some() {
        return;
    }
    let (_guard, scratch) = scratch();
    let page = scratch.join("user-config-fixture.md");
    std::fs::write(
        &page,
        "```console\n\
         $ diataxis store root\n\
         root_store: /docs-test-fixture-root ([PARENT]/home/.config/diataxis/config.yml)\n\
         \n\
         $ cat note.txt\n\
         from the work fixture\n\
         \n\
         $ diataxis --user-config ../home/other.yml store root\n\
         root_store: unset (../home/other.yml)\n\
         \n\
         ```\n",
    )
    .unwrap();
    run_page(&scratch, &page);
}

/// A page's commands see the page's `HOME` and XDG directories, never
/// the test process's, whatever the process sets.
#[test]
fn a_page_sees_its_own_home_and_xdg_directories() {
    if std::env::var_os("TRYCMD").is_some() {
        return;
    }
    let (_guard, scratch) = scratch();
    let page = scratch.join("page-env.md");
    std::fs::write(
        &page,
        "```console\n\
         $ printenv HOME\n\
         [PARENT]/home\n\
         \n\
         $ printenv XDG_CONFIG_HOME\n\
         [PARENT]/home/.config\n\
         \n\
         $ printenv XDG_STATE_HOME\n\
         [PARENT]/home/.local/state\n\
         \n\
         $ printenv XDG_DATA_HOME\n\
         [PARENT]/home/.local/share\n\
         \n\
         $ printenv XDG_CACHE_HOME\n\
         [PARENT]/home/.cache\n\
         \n\
         ```\n",
    )
    .unwrap();
    run_page(&scratch, &page);
}

/// A page's commands see none of the test process's environment.
#[test]
fn page_commands_run_in_a_clean_environment() {
    let (_guard, scratch) = scratch();
    let printenv = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|dir| dir.join("printenv"))
        .find(|p| p.is_file())
        .expect("printenv on PATH");
    let wrapper = write_wrapper(&scratch, "printenv", &printenv);
    let out = Command::new(&wrapper)
        .env("MDSTORE_REGISTRY", "/elsewhere")
        .env("NO_COLOR", "0")
        .env("HOME", "/page-home")
        .output()
        .unwrap();
    let env = String::from_utf8(out.stdout).unwrap();
    assert!(env.contains("HOME=/page-home\n"), "{env}");
    assert!(env.contains("NO_COLOR=1\n"), "{env}");
    assert!(!env.contains("MDSTORE_REGISTRY"), "{env}");
}

/// Every page the docs test runs: README.md, and each Markdown file in
/// docs/, counted here by name rather than by the filter `pages` uses.
#[test]
fn pages_cover_readme_and_every_docs_page() {
    let mut expected: Vec<PathBuf> = std::fs::read_dir(root().join("docs"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_string_lossy().ends_with(".md"))
        .collect();
    expected.sort();
    assert!(expected.len() >= 4, "{expected:?}");
    expected.insert(0, root().join("README.md"));
    assert_eq!(pages(), expected);
}

/// A `> ` line continues a command, so it may hold a backslash; an
/// output line may not.
#[test]
fn a_continuation_line_may_hold_a_backslash() {
    let names: Vec<String> = commands(
        Path::new("p.md"),
        "```console\n$ diataxis search\n> 'a\\b'\nok\n```\n",
    )
    .into_iter()
    .collect();
    assert_eq!(names, ["diataxis"]);
}

#[test]
fn commands_lists_what_trycmd_runs() {
    let page = "```sh\nls\n```\n\n````markdown\n```console\n$ inner\n```\n````\n\n```console\n$ A=1 diataxis check\n\n$ cat out.txt\nok\n```\n\n```\n$ bare\n```\n\n```trycmd\n$ tagged\n```\n";
    let names: Vec<String> = commands(Path::new("p.md"), page).into_iter().collect();
    assert_eq!(names, ["bare", "cat", "diataxis", "tagged"]);
}

/// The `--root` and `--home` help names the commands that ignore
/// `--root` and refuse `--home`. Each claim holds against the binary,
/// run outside any set with a scratch user config.
#[test]
fn the_root_and_home_help_holds() {
    let (_guard, scratch) = scratch();
    let config = scratch.join("config.yml");
    let run = |args: &[&str]| {
        Command::new(BIN)
            .arg("--user-config")
            .arg(&config)
            .args(args)
            .current_dir(&scratch)
            .env_clear()
            .env("HOME", &scratch)
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    };
    let rootless: [&[&str]; 4] = [&["prime"], &["docs"], &["store", "root"], &["compass"]];
    for args in rootless {
        let ignored = run(&[&["--root", "/no-such-set"], args].concat());
        assert!(ignored.status.success(), "{args:?} does not ignore --root");
        let refused = run(&[&["--home"], args].concat());
        assert!(!refused.status.success(), "{args:?} accepts --home");
        assert!(
            String::from_utf8_lossy(&refused.stderr).contains("--home does not apply"),
            "{args:?}"
        );
    }
    let init = run(&["--home", "init"]);
    assert!(!init.status.success(), "init accepts --home");
    assert!(!scratch.join("diataxis.yml").exists());
    // With a root set configured, a read outside any set falls back to it
    // and names it on stderr, and a write does not fall back.
    std::fs::create_dir(scratch.join("root")).unwrap();
    assert!(run(&["--root", "root", "init"]).status.success());
    assert!(run(&["store", "root", "root"]).status.success());
    let read = run(&["map"]);
    assert!(read.status.success(), "a read did not fall back");
    let stderr = String::from_utf8_lossy(&read.stderr);
    assert!(stderr.contains("reading root set"), "{stderr}");
    let write = run(&["page", "create", "A", "-T", "tutorial"]);
    assert!(!write.status.success(), "a write outside any set succeeded");
    // miette wraps the message at a point that moves with the scratch
    // path's length, behind a `|` gutter, so join the words first.
    let stderr = String::from_utf8_lossy(&write.stderr);
    let joined = stderr
        .split_whitespace()
        .filter(|w| *w != "|")
        .collect::<Vec<_>>()
        .join(" ");
    assert!(joined.contains("a write does not fall back"), "{stderr}");
    assert!(!scratch.join("root/docs/a.md").exists());
}

/// A quoted command word is the program trycmd runs, without its quotes.
#[test]
fn a_quoted_command_word_names_its_program() {
    let names: Vec<String> = commands(
        Path::new("p.md"),
        "```console\n$ \"diataxis\" check\n\n$ 'cat' a\n```\n",
    )
    .into_iter()
    .collect();
    assert_eq!(names, ["cat", "diataxis"]);
}

/// An unclosed quote still names the command, as trycmd still runs it, so
/// the command is registered rather than skipped or run unwrapped.
#[test]
fn a_line_with_an_unclosed_quote_still_names_its_command() {
    let names: Vec<String> = commands(
        Path::new("p.md"),
        "```console\n$ cat 'notes.txt\n\n$ diataxis search 'a b\n```\n",
    )
    .into_iter()
    .collect();
    assert_eq!(names, ["cat", "diataxis"]);
}

/// A fence closes only at its own backticks, so a shorter fence inside a
/// longer console block does not end it.
#[test]
fn a_console_block_ends_at_its_own_backticks() {
    let page = "````console\n$ first\n```\n$ second\n````\n";
    let names: Vec<String> = commands(Path::new("p.md"), page).into_iter().collect();
    assert_eq!(names, ["first", "second"]);
}

/// A file fence shown inside a longer fence, as an example of the
/// syntax, is not a file the reader writes: the outer fence closes only
/// at its own backticks.
#[test]
fn a_file_fence_inside_a_longer_fence_writes_nothing() {
    let page = "````markdown\n```\n```text {filename=\"a.txt\"}\nx\n```\n````\n";
    let segments = segments(Path::new("p.md"), page);
    assert_eq!(segments.len(), 1);
    assert!(segments[0].file.is_none());
}

#[test]
fn a_page_running_an_unknown_command_fails() {
    let (_guard, scratch) = scratch();
    let page = scratch.join("unknown.md");
    std::fs::write(&page, "```console\n$ no-such-command-in-path\n```\n").unwrap();
    let err = std::panic::catch_unwind(|| run_page(&scratch, &page)).unwrap_err();
    let message = err.downcast_ref::<String>().cloned().unwrap_or_default();
    assert!(message.contains("not on PATH"), "{message}");
}

#[test]
#[should_panic(expected = "hides its commands")]
fn an_ignored_console_block_is_refused() {
    commands(
        Path::new("p.md"),
        "```console,ignore\n$ diataxis check\n```\n",
    );
}

#[test]
#[should_panic(expected = "backslash")]
fn output_with_a_backslash_is_refused() {
    commands(
        Path::new("p.md"),
        "```console\n$ diataxis search x\na:1  how-to  \\[[x]]\n```\n",
    );
}

#[test]
#[should_panic(expected = "outside the working directory")]
fn a_file_fence_outside_the_working_directory_is_refused() {
    let (_guard, scratch) = scratch();
    let page = scratch.join("escape.md");
    std::fs::write(&page, "```text {filename=\"../x\"}\nx\n```\n").unwrap();
    run_page(&scratch, &page);
}

#[test]
#[should_panic(expected = "outside the working directory")]
fn a_file_fence_with_an_absolute_path_is_refused() {
    let (_guard, scratch) = scratch();
    let page = scratch.join("absolute.md");
    let target = scratch.join("escaped.txt");
    std::fs::write(
        &page,
        format!("```text {{filename=\"{}\"}}\nx\n```\n", target.display()),
    )
    .unwrap();
    run_page(&scratch, &page);
}

#[test]
#[should_panic(expected = "needs a language")]
fn a_file_fence_without_a_language_is_refused() {
    segments(Path::new("p.md"), "```{filename=\"a.txt\"}\nx\n```\n");
}

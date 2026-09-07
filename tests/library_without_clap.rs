//! The library half, driven the way a tool with its own argument parser
//! drives it.
//!
//! gaff parses arguments by hand, because clap exits 2 on a usage error
//! and gaff reserves 2 for a guard refusing a tool call. This test
//! stands in for that caller: it names files, asks for text, and keeps
//! its own exit codes.
//!
//! `DocSet`, `Request` and `docs` are the store half, so this whole file
//! needs that feature. Without the gate it failed under `--features
//! build` alone, which is how a build-dependency takes this crate.
#![cfg(feature = "store")]

use diataxis::{DocSet, Request};

const WHAT: &str = "<!-- metadata\ntitle: \"What is gaff?\"\ndescription: \"A context-lifecycle handler\"\ntype: explanation\n-->\n\ngaff runs guards on tool calls.\n";
const GUARDS: &str = "<!-- metadata\ntitle: \"How to write a guard\"\ndescription: \"Refuse a tool call by pattern\"\ntype: how-to\n-->\n\nAdd a guard to gaff.yml.\n";

fn set() -> DocSet {
    DocSet::from_embedded(&[
        ("../docs/what-is-gaff.md", WHAT),
        ("../docs/how-to-write-a-guard.md", GUARDS),
    ])
    .unwrap()
}

/// The caller decides the exit code; the library returns text.
fn exit_code_for(request: Request<'_>) -> u8 {
    match set().render(request) {
        Ok(text) => {
            print!("{text}");
            0
        }
        Err(e) => {
            eprintln!("gaff: {e}");
            1
        }
    }
}

#[test]
fn a_hand_written_parser_serves_every_request() {
    let args: Vec<String> = vec!["search".into(), "guard".into()];
    let request = diataxis::docs::request_from_args(&args).unwrap();
    assert!(
        set()
            .render(request)
            .unwrap()
            .contains("how-to-write-a-guard")
    );
}

#[test]
fn a_missing_topic_is_an_error_the_caller_grades() {
    assert_eq!(exit_code_for(Request::Show("absent")), 1);
    assert_eq!(exit_code_for(Request::List), 0);
}

#[test]
fn the_whole_set_comes_back_in_one_stream() {
    let text = set().render(Request::All).unwrap();
    assert!(text.contains("gaff runs guards on tool calls."));
    assert!(text.contains("Add a guard to gaff.yml."));
    assert!(text.contains("<!-- page: what-is-gaff (explanation) -->"));
}

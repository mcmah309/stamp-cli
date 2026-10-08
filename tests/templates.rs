#![cfg(unix)]

use expectrl::{Eof, Expect, Session, process::unix::WaitStatus};
use serde::Deserialize;
use std::{fs, process::Command, time::Duration};

mod common;

#[derive(Deserialize)]
struct Answer {
    prompt: String,
    answer: String,
    #[serde(default)]
    raw: bool,
    ready: Option<String>,
}

fn assert_template_renders(source: &str, answers: &str) {
    let answers: Vec<Answer> = serde_json::from_slice(
        &fs::read(common::template_path(&format!("answers/{answers}.json"))).unwrap(),
    )
    .unwrap();

    for registered in [false, true]
        .into_iter()
        .filter(|registered| !registered || cfg!(target_os = "linux"))
    {
        let directory = tempfile::tempdir().unwrap();
        let template = directory.path().join("source");
        let destination = directory.path().join("destination");
        common::copy_directory(&common::template_path(source), &template);
        let command = || {
            let mut command = Command::new(env!("CARGO_BIN_EXE_stamp"));
            command.env("XDG_CONFIG_HOME", directory.path().join("config"));
            command.env("TERM", "xterm");
            command
        };
        let mut render = command();
        if registered {
            let output = command().arg("register").arg(&template).output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            render.args(["use", "source"]);
        } else {
            render.arg("from").arg(&template);
        }
        render.arg(&destination);
        let mut session = Session::spawn(render).unwrap();
        session.set_expect_timeout(Some(Duration::from_secs(10)));
        for answer in &answers {
            session.expect(answer.prompt.as_str()).unwrap();
            session
                .expect(answer.ready.as_deref().unwrap_or("› "))
                .unwrap();
            if answer.raw {
                session.send(&answer.answer).unwrap();
            } else {
                session.send_line(&answer.answer).unwrap();
            }
        }
        session.expect("Template rendered successfully").unwrap();
        session.expect(Eof).unwrap();
        assert_eq!(
            session.get_process().wait().unwrap(),
            WaitStatus::Exited(session.get_process().pid(), 0)
        );
        common::assert_directory_eq(
            &destination,
            &common::template_path(&format!("rendered/{source}")),
        );
        drop(session);
        common::cleanup(directory);
    }
}

#[test]
fn axum_server_template_renders_contents_and_interpolated_paths() {
    assert_template_renders("axum_server", "axum_server");
}

#[test]
fn rust_devcontainer_template_renders_hidden_files() {
    assert_template_renders("devcontainers/rust", "rust");
}

#[test]
fn flutter_rust_devcontainer_template_renders_hidden_files() {
    assert_template_renders("devcontainers/flutter_rust", "flutter_rust");
}

#[test]
fn string_questions_accept_defaults_and_custom_values() {
    assert_template_renders("questions/string", "string");
}

#[test]
fn boolean_questions_accept_yes_no_and_defaults() {
    assert_template_renders("questions/bool", "bool");
}

#[test]
fn select_questions_accept_defaults_navigation_and_first_option_fallbacks() {
    assert_template_renders("questions/select", "select");
}

#[test]
fn multi_select_questions_support_toggles_defaults_empty_and_all_selections() {
    assert_template_renders("questions/multi-select", "multi-select");
}

#[test]
fn invalid_question_definitions_report_all_errors_before_rendering() {
    let fixture = common::template_path("questions/invalid");
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("destination");
    let output = Command::new(env!("CARGO_BIN_EXE_stamp"))
        .arg("from")
        .arg(fixture.join("incoming"))
        .arg(&destination)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    for expected in fs::read_to_string(fixture.join("expected-errors.txt"))
        .unwrap()
        .lines()
    {
        assert!(error.contains(expected), "Missing {expected:?} in {error}");
    }
    assert!(!destination.exists());
    common::cleanup(directory);
}

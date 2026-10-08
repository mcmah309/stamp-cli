use std::{
    path::PathBuf,
    process::{Command, Output},
};
use tempfile::TempDir;

mod common;

struct Fixture {
    directory: TempDir,
    source: PathBuf,
    destination: PathBuf,
    case: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let case = common::template_path(&format!("conflicts/{name}"));
        let source = case.join("incoming");
        let destination = directory.path().join("destination");
        common::copy_directory(&case.join("original"), &destination);
        Self {
            directory,
            source,
            destination,
            case,
        }
    }

    fn render(&self, flags: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_stamp"))
            .arg("from")
            .arg(&self.source)
            .arg(&self.destination)
            .args(flags)
            .output()
            .unwrap()
    }

    fn assert_expected(self, name: &str) {
        common::assert_directory_eq(&self.destination, &self.case.join(name));
        self.cleanup();
    }

    fn cleanup(self) {
        common::cleanup(self.directory);
    }
}

#[test]
fn noninteractive_conflicts_fail_before_writing_any_files() {
    let fixture = Fixture::new("no-terminal");
    let output = fixture.render(&[]);
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    for expected in [
        "interactive terminal",
        "z-existing.md",
        "--overwrite-conflicts",
        "--skip-conflicts",
        "--merge-overwrite-conflicts",
        "--merge-skip-conflicts",
    ] {
        assert!(error.contains(expected), "{error}");
    }
    fixture.assert_expected("original");
}

#[test]
fn overwrite_flag_replaces_entire_markdown_files_and_renders_templates() {
    let fixture = Fixture::new("overwrite");
    let output = fixture.render(&["--overwrite-conflicts"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fixture.assert_expected("expected-overwrite");
}

#[test]
fn skip_flag_keeps_conflicts_without_rendering_skipped_templates() {
    let fixture = Fixture::new("skip-invalid");
    let output = fixture.render(&["--skip-conflicts"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fixture.assert_expected("expected-skip");
}

#[test]
fn no_conflicts_need_no_terminal_or_flags() {
    let fixture = Fixture::new("no-conflicts");
    let output = fixture.render(&[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fixture.assert_expected("expected");
}

#[test]
fn conflict_flags_remain_mutually_exclusive() {
    let fixture = Fixture::new("no-conflicts");
    let flags = [
        "--overwrite-conflicts",
        "--skip-conflicts",
        "--merge-overwrite-conflicts",
        "--merge-skip-conflicts",
    ];
    for (index, flag) in flags.iter().enumerate() {
        for other in &flags[index + 1..] {
            for command in ["from", "use"] {
                let output = if command == "from" {
                    fixture.render(&[flag, other])
                } else {
                    Command::new(env!("CARGO_BIN_EXE_stamp"))
                        .args(["use", "template", flag, other])
                        .output()
                        .unwrap()
                };
                assert!(!output.status.success());
                assert!(String::from_utf8_lossy(&output.stderr).contains("cannot be used with"));
            }
        }
    }
    fixture.cleanup();
}

#[test]
fn merge_flags_apply_the_same_preference_to_every_conflicting_file_type() {
    for (flag, expected) in [
        ("--merge-overwrite-conflicts", "expected-merge-overwrite"),
        ("--merge-skip-conflicts", "expected-merge-skip"),
    ] {
        let fixture = Fixture::new("mixed");
        let output = fixture.render(&[flag]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        fixture.assert_expected(expected);
    }
}

#[test]
fn merges_rendered_template_content_at_the_rendered_destination_path() {
    for (flag, expected) in [
        ("--merge-overwrite-conflicts", "expected-merge-overwrite"),
        ("--merge-skip-conflicts", "expected-merge-skip"),
    ] {
        let fixture = Fixture::new("rendered-path");
        let output = fixture.render(&[flag]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        fixture.assert_expected(expected);
    }
}

#[test]
fn a_failed_merge_preparation_leaves_all_files_untouched() {
    for flag in ["--merge-overwrite-conflicts", "--merge-skip-conflicts"] {
        let fixture = Fixture::new("failed-merge");
        let output = fixture.render(&[flag]);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("z-invalid.json"));
        fixture.assert_expected("original");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn registered_templates_support_both_merge_flags() {
    for (flag, expected) in [
        ("--merge-overwrite-conflicts", "expected-merge-overwrite"),
        ("--merge-skip-conflicts", "expected-merge-skip"),
    ] {
        let fixture = Fixture::new("registered");
        let command = || {
            let mut command = Command::new(env!("CARGO_BIN_EXE_stamp"));
            command.env("XDG_CONFIG_HOME", fixture.directory.path().join("config"));
            command
        };
        let registered = command()
            .arg("register")
            .arg(&fixture.source)
            .output()
            .unwrap();
        assert!(
            registered.status.success(),
            "{}",
            String::from_utf8_lossy(&registered.stderr)
        );
        let output = command()
            .args(["use", "Conflict fixture"])
            .arg(&fixture.destination)
            .arg(flag)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        fixture.assert_expected(expected);
    }
}

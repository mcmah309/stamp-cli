use std::{fs, process::Command};

const DEBUG_HINT: &str =
    "Run again with --debug to show raw error details, context, and a backtrace.";

fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_stamp"))
}

#[test]
fn normal_errors_stay_clean_even_when_backtraces_are_enabled() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("missing");
    let output = command()
        .arg("from")
        .arg(&source)
        .arg(directory.path().join("destination"))
        .env("RUST_BACKTRACE", "full")
        .env("RUST_LIB_BACKTRACE", "1")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        format!(
            "Error: Could not read `{}`. Check that the template exists and is readable.\n\n{DEBUG_HINT}\n",
            source.join("stamp.toml").display()
        )
    );
}

#[test]
fn debug_flag_is_global_and_enables_backtraces_without_environment_setup() {
    let directory = tempfile::tempdir().unwrap();
    for before_command in [true, false] {
        for disabled in [false, true] {
            let mut command = command();
            if before_command {
                command.arg("--debug");
            }
            command
                .arg("from")
                .arg(directory.path().join("missing"))
                .arg(directory.path().join("destination"));
            if !before_command {
                command.arg("--debug");
            }
            if disabled {
                command.env("RUST_BACKTRACE", "0");
                command.env("RUST_LIB_BACKTRACE", "0");
            } else {
                command.env_remove("RUST_BACKTRACE");
                command.env_remove("RUST_LIB_BACKTRACE");
            }
            let output = command.output().unwrap();
            let error = String::from_utf8(output.stderr).unwrap();
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            assert!(error.starts_with("Error: Could not read"), "{error}");
            for expected in [
                "Debug details:",
                "Context (innermost first):",
                "Backtrace (captured):",
                "stamp::render_template",
            ] {
                assert!(error.contains(expected), "Missing {expected:?}: {error}");
            }
            assert!(!error.contains(DEBUG_HINT), "{error}");
        }
    }
}

#[test]
fn template_diagnostics_are_available_only_in_debug_output() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("stamp.toml"), "").unwrap();
    fs::write(source.join("file.txt.tera"), "{{ missing_variable }}").unwrap();

    for debug in [false, true] {
        let mut command = command();
        command
            .arg("from")
            .arg(&source)
            .arg(directory.path().join("destination"));
        if debug {
            command.arg("--debug");
        }
        let output = command.output().unwrap();
        let error = String::from_utf8(output.stderr).unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(error.contains("Could not render template file"), "{error}");
        assert!(
            error.contains("Check its template syntax and variables."),
            "{error}"
        );
        assert_eq!(error.contains("missing_variable"), debug, "{error}");
        assert_eq!(error.contains("Backtrace (captured):"), debug, "{error}");
    }
}

#[test]
fn debug_path_rendering_retains_the_original_template_error() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("stamp.toml"), "").unwrap();
    fs::write(source.join("{{ missing_variable }}.txt"), "contents").unwrap();
    let output = command()
        .arg("from")
        .arg(&source)
        .arg(directory.path().join("destination"))
        .arg("--debug")
        .output()
        .unwrap();
    let error = String::from_utf8(output.stderr).unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(error.contains("Failed to render path component"), "{error}");
    let details = error.split_once("Debug details:\n").unwrap().1;
    assert!(
        details.lines().next().unwrap().contains("missing_variable"),
        "{error}"
    );
    assert!(!directory.path().join("destination").exists());
}

#[cfg(target_os = "linux")]
#[test]
fn registry_errors_hide_internal_context_and_recognize_application_errors() {
    let directory = tempfile::tempdir().unwrap();
    let registry = directory.path().join("stamp/template_registry.json");
    fs::create_dir(registry.parent().unwrap()).unwrap();

    let output = command()
        .args(["use", "missing"])
        .env("XDG_CONFIG_HOME", directory.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        format!(
            "Error: Template 'missing' not found in registry. Run `stamp list` to see available templates.\n\n{DEBUG_HINT}\n"
        )
    );

    fs::write(&registry, "invalid json").unwrap();
    for debug in [false, true] {
        let mut command = command();
        command.arg("list").env("XDG_CONFIG_HOME", directory.path());
        if debug {
            command.arg("--debug");
        }
        let output = command.output().unwrap();
        let error = String::from_utf8(output.stderr).unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(
            error.contains("The template registry is invalid."),
            "{error}"
        );
        assert_eq!(
            error.contains(&registry.display().to_string()),
            debug,
            "{error}"
        );
    }
}

#[test]
fn debug_flag_is_documented_for_every_command() {
    for subcommand in [
        None,
        Some("use"),
        Some("from"),
        Some("register"),
        Some("remove"),
        Some("list"),
    ] {
        let mut command = command();
        if let Some(subcommand) = subcommand {
            command.arg(subcommand);
        }
        let output = command.arg("--help").output().unwrap();
        assert!(output.status.success());
        let help = String::from_utf8(output.stdout).unwrap();
        assert!(help.contains("--debug"), "{help}");
        assert!(
            help.contains("Show raw error details, context, and a backtrace"),
            "{help}"
        );
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn successful_commands_with_debug_produce_no_error_output() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("stamp.toml"), "").unwrap();
    fs::write(source.join("file.txt"), "contents").unwrap();
    let destination = directory.path().join("destination");
    let output = command()
        .arg("from")
        .arg(&source)
        .arg(&destination)
        .arg("--debug")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(
        fs::read_to_string(destination.join("file.txt")).unwrap(),
        "contents"
    );
}

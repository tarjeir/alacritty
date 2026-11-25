use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn migrate(dir: &Path, config: &str, dry_run: bool) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_alacritty"));
    cmd.arg("migrate").arg("--config-file").arg(dir.join(config));
    if dry_run {
        cmd.arg("--dry-run");
    }
    cmd.env("HOME", dir).env("XDG_CONFIG_HOME", dir).current_dir(dir).output().unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

const LEGACY_TOML: &str = "key_bindings = []\nlive_config_reload = true\n";

#[test]
fn migrate_toml_wet_run() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("alacritty.toml"), LEGACY_TOML).unwrap();

    let output = migrate(dir.path(), "alacritty.toml", false);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(stdout(&output).contains("Successfully migrated"));
    let migrated = fs::read_to_string(dir.path().join("alacritty.toml")).unwrap();
    assert!(migrated.contains("[keyboard]"));
    assert!(migrated.contains("[general]"));
    assert!(!migrated.contains("key_bindings"));
}

#[test]
fn migrate_toml_dry_run() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("alacritty.toml"), LEGACY_TOML).unwrap();

    let output = migrate(dir.path(), "alacritty.toml", true);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(stdout(&output).contains("Start TOML"));
    assert_eq!(fs::read_to_string(dir.path().join("alacritty.toml")).unwrap(), LEGACY_TOML);
}

#[test]
fn migrate_yaml_rejected() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("alacritty.yml"), "window: {}\n").unwrap();

    let output = migrate(dir.path(), "alacritty.yml", false);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("no longer supported"), "{}", stderr(&output));
}

#[test]
fn migrate_missing_import_skipped() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("alacritty.toml"), "[general]\nimport = [\"missing.toml\"]\n")
        .unwrap();

    let output = migrate(dir.path(), "alacritty.toml", true);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(stdout(&output).contains("Skipping migration for nonexistent path"));
}

#[test]
fn migrate_import_depth_limit() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..7 {
        let contents = format!("[general]\nimport = [\"chain{}.toml\"]\n", i + 1);
        fs::write(dir.path().join(format!("chain{i}.toml")), contents).unwrap();
    }
    fs::write(dir.path().join("chain7.toml"), "").unwrap();

    let output = migrate(dir.path(), "chain0.toml", true);

    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("Exceeded maximum configuration import depth"),
        "{}",
        stderr(&output)
    );
}

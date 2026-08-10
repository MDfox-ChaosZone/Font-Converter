use std::{fs, path::Path, process::Command};

use serde_json::Value;

fn cli() -> &'static str {
    env!("CARGO_BIN_EXE_font-converter-cli")
}

#[test]
fn dry_run_filters_mode_and_preserves_relative_directories() {
    let input = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let nested = input.path().join("family").join("weight");
    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join("font.ttf"), b"fixture").unwrap();
    fs::write(nested.join("ignored.woff2"), b"wOF2\0\x01\0\0").unwrap();

    let result = Command::new(cli())
        .args(["--dry-run", "--json", "--mode", "encode", "-o"])
        .arg(output.path())
        .arg(input.path())
        .output()
        .unwrap();

    assert!(result.status.success());
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["dryRun"], true);
    assert_eq!(report["summary"]["total"], 1);
    assert_eq!(report["summary"]["queued"], 1);
    let output_path = report["items"][0]["outputPath"].as_str().unwrap();
    assert!(Path::new(output_path).ends_with(Path::new("family/weight/font.woff2")));
    assert!(!output.path().join("family/weight/font.woff2").exists());
}

#[test]
fn strict_mode_fails_on_scan_warnings() {
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("missing.ttf");

    let result = Command::new(cli())
        .args(["--dry-run", "--strict", "--json"])
        .arg(missing)
        .output()
        .unwrap();

    assert_eq!(result.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["warnings"].as_array().unwrap().len(), 1);
}

#[test]
fn existing_output_policies_are_reported_without_writing_in_dry_run() {
    let input = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let font = input.path().join("font.ttf");
    let existing = output.path().join("font.woff2");
    fs::write(&font, b"fixture").unwrap();
    fs::write(&existing, b"keep me").unwrap();

    for (policy, expected_status, expected_exit) in [
        ("skip", "skipped", 0),
        ("error", "failed", 1),
        ("overwrite", "queued", 0),
    ] {
        let result = Command::new(cli())
            .args(["--dry-run", "--json", "--existing", policy, "-o"])
            .arg(output.path())
            .arg(&font)
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(expected_exit));
        let report: Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(report["items"][0]["status"], expected_status);
        assert_eq!(fs::read(&existing).unwrap(), b"keep me");
    }
}

#[test]
fn rejects_zero_jobs() {
    let result = Command::new(cli())
        .args(["--jobs", "0", "fixture.ttf"])
        .output()
        .unwrap();

    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains("jobs must be between 1 and 256"));
}

#[test]
fn converts_real_fonts_in_parallel_when_configured() {
    let Some(fixture) = std::env::var_os("FONT_CONVERTER_TEST_FONT") else {
        return;
    };
    let input = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    for family in ["first", "second"] {
        let directory = input.path().join(family);
        fs::create_dir(&directory).unwrap();
        fs::copy(&fixture, directory.join("font.ttf")).unwrap();
    }

    let result = Command::new(cli())
        .args(["--json", "--mode", "encode", "--jobs", "2", "-o"])
        .arg(output.path())
        .arg(input.path())
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["summary"]["succeeded"], 2);
    assert!(output.path().join("first/font.woff2").exists());
    assert!(output.path().join("second/font.woff2").exists());
}

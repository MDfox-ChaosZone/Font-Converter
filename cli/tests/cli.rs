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
    assert_eq!(report["formatVersion"], 1);
    assert_eq!(report["dryRun"], true);
    assert_eq!(report["summary"]["total"], 1);
    assert_eq!(report["summary"]["queued"], 1);
    assert_eq!(report["items"][0]["id"], "1");
    let output_path = report["items"][0]["outputPath"].as_str().unwrap();
    assert!(Path::new(output_path).ends_with(Path::new("family/weight/font.woff2")));
    assert!(!output.path().join("family/weight/font.woff2").exists());
}

#[test]
fn scan_warnings_fail_when_valid_items_are_also_present() {
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("missing.ttf");
    let valid = directory.path().join("valid.ttf");
    fs::write(&valid, b"fixture").unwrap();

    let result = Command::new(cli())
        .args(["--dry-run", "--json"])
        .arg(valid)
        .arg(missing)
        .output()
        .unwrap();

    assert_eq!(result.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["warnings"].as_array().unwrap().len(), 1);
    assert_eq!(report["warnings"][0]["errorCode"], "input_not_found");
}

#[test]
fn no_supported_input_uses_exit_code_two() {
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("missing.ttf");

    let result = Command::new(cli())
        .args(["--dry-run", "--json"])
        .arg(missing)
        .output()
        .unwrap();

    assert_eq!(result.status.code(), Some(2));
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["summary"]["total"], 0);
    assert_eq!(report["warnings"][0]["errorCode"], "input_not_found");
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
        let expected_error = match policy {
            "skip" | "error" => Value::String("output_exists".into()),
            "overwrite" => Value::Null,
            _ => unreachable!(),
        };
        assert_eq!(report["items"][0]["errorCode"], expected_error);
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
    assert!(String::from_utf8_lossy(&result.stderr).contains("jobs must be between 1 and 32"));
}

#[test]
fn removed_options_are_rejected() {
    for option in ["--strict", "--quiet"] {
        let result = Command::new(cli())
            .args([option, "fixture.ttf"])
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(2));
    }
}

#[test]
fn dry_run_accepts_a_missing_output_directory_without_creating_it() {
    let input = tempfile::tempdir().unwrap();
    let output = input.path().join("new-output");
    let font = input.path().join("font.ttf");
    fs::write(&font, b"fixture").unwrap();

    let result = Command::new(cli())
        .args(["--dry-run", "--json", "-o"])
        .arg(&output)
        .arg(&font)
        .output()
        .unwrap();

    assert!(result.status.success());
    assert!(!output.exists());
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert!(Path::new(report["items"][0]["outputPath"].as_str().unwrap()).starts_with(&output));
}

#[test]
fn invalid_font_has_a_stable_error_code() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("broken.ttf");
    fs::write(&input, b"not a font").unwrap();

    let result = Command::new(cli())
        .args(["--json"])
        .arg(&input)
        .output()
        .unwrap();

    assert_eq!(result.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["items"][0]["status"], "failed");
    assert_eq!(report["items"][0]["errorCode"], "invalid_font");
    assert!(!input.with_extension("woff2").exists());
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

#[test]
fn conversion_creates_a_missing_output_directory() {
    let Some(fixture) = std::env::var_os("FONT_CONVERTER_TEST_FONT") else {
        return;
    };
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("created").join("nested");

    let result = Command::new(cli())
        .args(["--json", "--mode", "encode", "-o"])
        .arg(&output)
        .arg(fixture)
        .output()
        .unwrap();

    assert!(result.status.success());
    assert!(output.is_dir());
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["summary"]["succeeded"], 1);
}

#[test]
fn cli_decodes_a_real_woff2_back_to_ttf() {
    let Some(fixture) = std::env::var_os("FONT_CONVERTER_TEST_FONT") else {
        return;
    };
    let directory = tempfile::tempdir().unwrap();
    let encoded = directory.path().join("encoded");
    let decoded = directory.path().join("decoded");

    let encode = Command::new(cli())
        .args(["--json", "--mode", "encode", "-o"])
        .arg(&encoded)
        .arg(&fixture)
        .output()
        .unwrap();
    assert!(encode.status.success());
    let encode_report: Value = serde_json::from_slice(&encode.stdout).unwrap();
    let woff2 = encode_report["items"][0]["outputPath"].as_str().unwrap();

    let decode = Command::new(cli())
        .args(["--json", "--mode", "decode", "-o"])
        .arg(&decoded)
        .arg(woff2)
        .output()
        .unwrap();

    assert!(decode.status.success());
    let decode_report: Value = serde_json::from_slice(&decode.stdout).unwrap();
    assert_eq!(decode_report["summary"]["succeeded"], 1);
    assert!(Path::new(decode_report["items"][0]["outputPath"].as_str().unwrap()).exists());
}

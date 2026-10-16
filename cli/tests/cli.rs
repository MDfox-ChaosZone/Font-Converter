use std::{
    path::{Path, PathBuf},
    process::Command,
};
fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_font-converter-cli"))
}
fn fixture() -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/Abel-Regular.ttf");
    assert!(path.exists());
    path
}
#[test]
fn dry_run_reports_v2_and_does_not_write() {
    let temp = tempfile::tempdir().unwrap();
    let dest = temp.path().join("missing");
    let result = cli()
        .args(["--json", "--dry-run", "-o"])
        .arg(&dest)
        .arg(fixture())
        .output()
        .unwrap();
    assert!(result.status.success());
    assert!(result.stderr.is_empty());
    assert!(!dest.exists());
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["formatVersion"], 2);
    assert_eq!(report["summary"]["queued"], 1);
    assert_eq!(report["dryRun"], true);
    assert!(report["options"].is_object());
}
#[test]
fn conversion_skip_error_and_decode() {
    let temp = tempfile::tempdir().unwrap();
    let run = |policy: &str| {
        cli()
            .args(["--json", "--existing", policy, "-o"])
            .arg(temp.path())
            .arg(fixture())
            .output()
            .unwrap()
    };
    let first = run("skip");
    assert!(first.status.success());
    let skipped: serde_json::Value = serde_json::from_slice(&run("skip").stdout).unwrap();
    assert_eq!(skipped["summary"]["skipped"], 1);
    assert_eq!(run("error").status.code(), Some(1));
    assert!(run("overwrite").status.success());
    let decoded = cli()
        .args(["--mode", "decode", "--json", "-o"])
        .arg(temp.path().join("decoded"))
        .arg(temp.path().join("Abel-Regular.woff2"))
        .output()
        .unwrap();
    assert!(decoded.status.success());
}
#[test]
fn exit_codes_and_machine_errors() {
    assert_eq!(
        cli()
            .args(["--jobs", "0"])
            .arg(fixture())
            .status()
            .unwrap()
            .code(),
        Some(2)
    );
    let missing = cli()
        .args(["--json", "a-missing-font.ttf"])
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&missing.stdout).unwrap();
    assert_eq!(report["warnings"][0]["errorCode"], "input_not_found");
    let temp = tempfile::tempdir().unwrap();
    assert_eq!(cli().arg(temp.path()).status().unwrap().code(), Some(2));
}

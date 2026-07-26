use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use ttf2woff2_gui_shared::{ItemStatus, QueueItem, ScanResult, ScanWarning};
use uuid::Uuid;
use walkdir::WalkDir;

pub fn collect(paths: &[String]) -> ScanResult {
    let mut result = ScanResult::default();
    let mut seen = HashSet::new();

    for raw_path in paths {
        let path = PathBuf::from(raw_path);
        if path.is_dir() {
            collect_directory(&path, &mut seen, &mut result);
        } else if path.is_file() {
            collect_file(&path, &mut seen, &mut result);
        } else {
            result
                .warnings
                .push(warning(&path, "Path does not exist or cannot be accessed"));
        }
    }

    result
        .items
        .sort_by_key(|item| item.input_path.to_lowercase());
    result
}

fn collect_directory(path: &Path, seen: &mut HashSet<PathBuf>, result: &mut ScanResult) {
    for entry in WalkDir::new(path).follow_links(false) {
        match entry {
            Ok(entry) if entry.file_type().is_file() => {
                if is_ttf(entry.path()) {
                    collect_file(entry.path(), seen, result);
                }
            }
            Ok(_) => {}
            Err(error) => result.warnings.push(ScanWarning {
                path: error.path().unwrap_or(path).to_string_lossy().into_owned(),
                message: error.to_string(),
            }),
        }
    }
}

fn collect_file(path: &Path, seen: &mut HashSet<PathBuf>, result: &mut ScanResult) {
    if !is_ttf(path) {
        result
            .warnings
            .push(warning(path, "Only .ttf files are supported"));
        return;
    }

    let canonical = match fs::canonicalize(path) {
        Ok(path) => path,
        Err(error) => {
            result.warnings.push(warning(path, &error.to_string()));
            return;
        }
    };
    if !seen.insert(canonical.clone()) {
        return;
    }

    let output = canonical.with_extension("woff2");
    let output_exists = output.exists();
    let output_bytes = output_exists
        .then(|| fs::metadata(&output).ok().map(|metadata| metadata.len()))
        .flatten();
    result.items.push(QueueItem {
        id: Uuid::new_v4().to_string(),
        input_path: canonical.to_string_lossy().into_owned(),
        output_path: output.to_string_lossy().into_owned(),
        input_bytes: fs::metadata(&canonical).ok().map(|metadata| metadata.len()),
        output_bytes,
        status: if output_exists {
            ItemStatus::Skipped
        } else {
            ItemStatus::Queued
        },
        message: output_exists.then(|| "Output file already exists".into()),
    });
}

fn is_ttf(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("ttf"))
}

fn warning(path: &Path, message: &str) -> ScanWarning {
    ScanWarning {
        path: path.to_string_lossy().into_owned(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recursively_collects_case_insensitive_ttf_and_deduplicates() {
        let directory = tempfile::tempdir().unwrap();
        let nested = directory.path().join("nested");
        fs::create_dir(&nested).unwrap();
        let font = nested.join("字体.TTF");
        fs::write(&font, b"fixture").unwrap();
        fs::write(nested.join("ignored.otf"), b"fixture").unwrap();

        let result = collect(&[
            directory.path().to_string_lossy().into_owned(),
            font.to_string_lossy().into_owned(),
        ]);

        assert_eq!(result.items.len(), 1);
        assert!(result.warnings.is_empty());
        assert_eq!(result.items[0].status, ItemStatus::Queued);
    }

    #[test]
    fn reports_explicit_unsupported_and_missing_paths() {
        let directory = tempfile::tempdir().unwrap();
        let otf = directory.path().join("font.otf");
        fs::write(&otf, b"fixture").unwrap();

        let result = collect(&[
            otf.to_string_lossy().into_owned(),
            directory
                .path()
                .join("missing.ttf")
                .to_string_lossy()
                .into_owned(),
        ]);

        assert!(result.items.is_empty());
        assert_eq!(result.warnings.len(), 2);
    }

    #[test]
    fn marks_existing_output_as_skipped() {
        let directory = tempfile::tempdir().unwrap();
        let font = directory.path().join("font.ttf");
        fs::write(&font, b"fixture").unwrap();
        fs::write(directory.path().join("font.woff2"), b"existing").unwrap();

        let result = collect(&[font.to_string_lossy().into_owned()]);

        assert_eq!(result.items[0].status, ItemStatus::Skipped);
        assert_eq!(result.items[0].output_bytes, Some(8));
    }
}

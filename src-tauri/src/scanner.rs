use std::{
    collections::HashSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use ttf2woff2_gui_shared::{ConversionKind, ItemStatus, QueueItem, ScanResult, ScanWarning};
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
                if is_supported_extension(entry.path()) {
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
    if !is_supported_extension(path) {
        result.warnings.push(warning(
            path,
            "Only .ttf, .otf, and .woff2 files are supported",
        ));
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

    let (conversion, output) = match conversion_for(&canonical) {
        Ok(conversion) => conversion,
        Err(message) => {
            result.warnings.push(warning(&canonical, &message));
            return;
        }
    };
    let output_exists = output.exists();
    let output_bytes = output_exists
        .then(|| fs::metadata(&output).ok().map(|metadata| metadata.len()))
        .flatten();
    result.items.push(QueueItem {
        id: Uuid::new_v4().to_string(),
        conversion,
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

fn is_supported_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            ["ttf", "otf", "woff2"]
                .iter()
                .any(|supported| extension.eq_ignore_ascii_case(supported))
        })
}

pub(crate) fn conversion_for(path: &Path) -> Result<(ConversionKind, PathBuf), String> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    if extension.eq_ignore_ascii_case("ttf") {
        return Ok((ConversionKind::TtfToWoff2, path.with_extension("woff2")));
    }
    if extension.eq_ignore_ascii_case("otf") {
        return Ok((ConversionKind::OtfToWoff2, path.with_extension("woff2")));
    }
    if !extension.eq_ignore_ascii_case("woff2") {
        return Err("Unsupported font extension".into());
    }

    let mut header = [0_u8; 8];
    fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut header))
        .map_err(|error| format!("Cannot read WOFF2 header: {error}"))?;
    if &header[..4] != b"wOF2" {
        return Err("The file does not contain a valid WOFF2 signature".into());
    }

    let (kind, extension) = match &header[4..8] {
        b"OTTO" | b"typ1" => (ConversionKind::Woff2ToOtf, "otf"),
        b"\0\x01\0\0" | b"true" => (ConversionKind::Woff2ToTtf, "ttf"),
        b"ttcf" => return Err("WOFF2 font collections are not supported".into()),
        _ => return Err("WOFF2 contains an unsupported SFNT flavor".into()),
    };
    Ok((kind, path.with_extension(extension)))
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
        fs::write(nested.join("font.otf"), b"fixture").unwrap();

        let result = collect(&[
            directory.path().to_string_lossy().into_owned(),
            font.to_string_lossy().into_owned(),
        ]);

        assert_eq!(result.items.len(), 2);
        assert!(result.warnings.is_empty());
        assert_eq!(result.items[0].status, ItemStatus::Queued);
    }

    #[test]
    fn reports_explicit_unsupported_and_missing_paths() {
        let directory = tempfile::tempdir().unwrap();
        let unsupported = directory.path().join("font.woff");
        fs::write(&unsupported, b"fixture").unwrap();

        let result = collect(&[
            unsupported.to_string_lossy().into_owned(),
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
    fn detects_woff2_output_type_from_flavor() {
        let directory = tempfile::tempdir().unwrap();
        let cff = directory.path().join("cff.woff2");
        let truetype = directory.path().join("truetype.WOFF2");
        fs::write(&cff, b"wOF2OTTO").unwrap();
        fs::write(&truetype, b"wOF2\0\x01\0\0").unwrap();

        let result = collect(&[
            cff.to_string_lossy().into_owned(),
            truetype.to_string_lossy().into_owned(),
        ]);

        assert!(result.warnings.is_empty());
        assert!(result.items.iter().any(|item| {
            item.conversion == ConversionKind::Woff2ToOtf && item.output_path.ends_with("cff.otf")
        }));
        assert!(result.items.iter().any(|item| {
            item.conversion == ConversionKind::Woff2ToTtf
                && item.output_path.ends_with("truetype.ttf")
        }));
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

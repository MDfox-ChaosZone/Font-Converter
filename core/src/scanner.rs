use std::{
    collections::HashSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use font_converter_shared::{
    ConversionKind, FolderConversionMode, ItemStatus, QueueItem, ScanResult, ScanWarning,
};
use uuid::Uuid;
use walkdir::WalkDir;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExistingOutputPolicy {
    Skip,
    Error,
    Overwrite,
}

#[derive(Clone, Copy, Debug)]
struct CollectOptions<'a> {
    output_directory: Option<&'a Path>,
    folder_mode: FolderConversionMode,
    filter_explicit_files: bool,
    preserve_directory_structure: bool,
    existing_output: ExistingOutputPolicy,
    output_conflicts_are_errors: bool,
}

#[derive(Clone, Copy, Debug)]
struct FileOptions<'a> {
    folder_mode: Option<FolderConversionMode>,
    relative_directory: Option<&'a Path>,
    existing_output: ExistingOutputPolicy,
    output_conflicts_are_errors: bool,
}

pub fn collect(paths: &[String], output_directory: Option<&Path>) -> ScanResult {
    collect_with_folder_mode(paths, output_directory, FolderConversionMode::Both)
}

pub fn collect_with_folder_mode(
    paths: &[String],
    output_directory: Option<&Path>,
    folder_mode: FolderConversionMode,
) -> ScanResult {
    collect_with_options(
        paths,
        CollectOptions {
            output_directory,
            folder_mode,
            filter_explicit_files: false,
            preserve_directory_structure: false,
            existing_output: ExistingOutputPolicy::Skip,
            output_conflicts_are_errors: false,
        },
    )
}

pub fn collect_for_cli(
    paths: &[String],
    output_directory: Option<&Path>,
    mode: FolderConversionMode,
    existing_output: ExistingOutputPolicy,
) -> ScanResult {
    collect_with_options(
        paths,
        CollectOptions {
            output_directory,
            folder_mode: mode,
            filter_explicit_files: true,
            preserve_directory_structure: true,
            existing_output,
            output_conflicts_are_errors: true,
        },
    )
}

fn collect_with_options(paths: &[String], options: CollectOptions<'_>) -> ScanResult {
    let mut result = ScanResult::default();
    let mut seen = HashSet::new();
    let mut seen_outputs = HashSet::new();
    let output_directory = match options.output_directory {
        Some(path) => match fs::canonicalize(path) {
            Ok(path) if path.is_dir() => Some(path),
            Ok(_) => {
                result
                    .warnings
                    .push(warning(path, "The output path is not a directory"));
                return result;
            }
            Err(error) => {
                result.warnings.push(warning(
                    path,
                    &format!("Cannot access the output directory: {error}"),
                ));
                return result;
            }
        },
        None => None,
    };

    for raw_path in paths {
        let path = PathBuf::from(raw_path);
        if path.is_dir() {
            let scan_root = fs::canonicalize(&path).unwrap_or(path);
            collect_directory(
                &scan_root,
                output_directory.as_deref(),
                &mut seen,
                &mut seen_outputs,
                &mut result,
                options,
            );
        } else if path.is_file() {
            collect_file(
                &path,
                output_directory.as_deref(),
                &mut seen,
                &mut seen_outputs,
                &mut result,
                FileOptions {
                    folder_mode: options.filter_explicit_files.then_some(options.folder_mode),
                    relative_directory: None,
                    existing_output: options.existing_output,
                    output_conflicts_are_errors: options.output_conflicts_are_errors,
                },
            );
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

fn collect_directory(
    path: &Path,
    output_directory: Option<&Path>,
    seen: &mut HashSet<PathBuf>,
    seen_outputs: &mut HashSet<String>,
    result: &mut ScanResult,
    options: CollectOptions<'_>,
) {
    let nested_output =
        output_directory.filter(|directory| directory.starts_with(path) && *directory != path);
    let entries = WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| nested_output != Some(entry.path()));
    for entry in entries {
        match entry {
            Ok(entry) if entry.file_type().is_file() => {
                if is_supported_extension(entry.path()) {
                    collect_file(
                        entry.path(),
                        output_directory,
                        seen,
                        seen_outputs,
                        result,
                        FileOptions {
                            folder_mode: Some(options.folder_mode),
                            relative_directory: options.preserve_directory_structure.then(|| {
                                entry
                                    .path()
                                    .strip_prefix(path)
                                    .unwrap_or(entry.path())
                                    .parent()
                                    .unwrap_or_else(|| Path::new(""))
                            }),
                            existing_output: options.existing_output,
                            output_conflicts_are_errors: options.output_conflicts_are_errors,
                        },
                    );
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

fn collect_file(
    path: &Path,
    output_directory: Option<&Path>,
    seen: &mut HashSet<PathBuf>,
    seen_outputs: &mut HashSet<String>,
    result: &mut ScanResult,
    options: FileOptions<'_>,
) {
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

    let (conversion, default_output) = match conversion_for(&canonical) {
        Ok(conversion) => conversion,
        Err(message) => {
            result.warnings.push(warning(&canonical, &message));
            return;
        }
    };
    if options
        .folder_mode
        .is_some_and(|mode| !mode.accepts(conversion))
    {
        return;
    }
    let output = match output_directory {
        Some(directory) => {
            let Some(file_name) = default_output.file_name() else {
                result
                    .warnings
                    .push(warning(&canonical, "Cannot determine the output file name"));
                return;
            };
            directory
                .join(options.relative_directory.unwrap_or_else(|| Path::new("")))
                .join(file_name)
        }
        None => default_output,
    };
    let output_key = output.to_string_lossy().to_lowercase();
    let output_conflicts = !seen_outputs.insert(output_key.clone());
    if output_conflicts
        && options.output_conflicts_are_errors
        && let Some(existing_item) = result
            .items
            .iter_mut()
            .find(|item| item.output_path.to_lowercase() == output_key)
    {
        existing_item.status = ItemStatus::Failed;
        existing_item.message = Some("Output path conflicts with another input font".into());
    }
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
        status: if output_conflicts && options.output_conflicts_are_errors {
            ItemStatus::Failed
        } else if output_conflicts {
            ItemStatus::Skipped
        } else if output_exists {
            match options.existing_output {
                ExistingOutputPolicy::Skip => ItemStatus::Skipped,
                ExistingOutputPolicy::Error => ItemStatus::Failed,
                ExistingOutputPolicy::Overwrite => ItemStatus::Queued,
            }
        } else {
            ItemStatus::Queued
        },
        message: if output_conflicts {
            Some(
                if options.output_conflicts_are_errors {
                    "Output path conflicts with another input font"
                } else {
                    "Output path conflicts with another queued font"
                }
                .into(),
            )
        } else {
            output_exists.then(|| match options.existing_output {
                ExistingOutputPolicy::Skip => "Output file already exists".into(),
                ExistingOutputPolicy::Error => {
                    "Output file already exists and policy is error".into()
                }
                ExistingOutputPolicy::Overwrite => "Existing output will be replaced".into(),
            })
        },
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

pub fn conversion_for(path: &Path) -> Result<(ConversionKind, PathBuf), String> {
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

#[cfg(test)]
pub(crate) fn is_valid_output_for(input: &Path, output: &Path, conversion: ConversionKind) -> bool {
    conversion_for(input).is_ok_and(|(expected_kind, expected_output)| {
        expected_kind == conversion && expected_output.file_name() == output.file_name()
    })
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

        let result = collect(
            &[
                directory.path().to_string_lossy().into_owned(),
                font.to_string_lossy().into_owned(),
            ],
            None,
        );

        assert_eq!(result.items.len(), 2);
        assert!(result.warnings.is_empty());
        assert_eq!(result.items[0].status, ItemStatus::Queued);
    }

    #[test]
    fn filters_directory_items_by_selected_conversion_mode() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("font.ttf"), b"fixture").unwrap();
        fs::write(directory.path().join("font.otf"), b"fixture").unwrap();
        fs::write(directory.path().join("cff.woff2"), b"wOF2OTTO").unwrap();
        let paths = &[directory.path().to_string_lossy().into_owned()];

        let font_to_woff2 =
            collect_with_folder_mode(paths, None, FolderConversionMode::FontToWoff2);
        assert_eq!(font_to_woff2.items.len(), 2);
        assert!(font_to_woff2.items.iter().all(|item| {
            matches!(
                item.conversion,
                ConversionKind::TtfToWoff2 | ConversionKind::OtfToWoff2
            )
        }));

        let woff2_to_font =
            collect_with_folder_mode(paths, None, FolderConversionMode::Woff2ToFont);
        assert_eq!(woff2_to_font.items.len(), 1);
        assert_eq!(
            woff2_to_font.items[0].conversion,
            ConversionKind::Woff2ToOtf
        );

        let both = collect_with_folder_mode(paths, None, FolderConversionMode::Both);
        assert_eq!(both.items.len(), 3);
    }

    #[test]
    fn reports_explicit_unsupported_and_missing_paths() {
        let directory = tempfile::tempdir().unwrap();
        let unsupported = directory.path().join("font.woff");
        fs::write(&unsupported, b"fixture").unwrap();

        let result = collect(
            &[
                unsupported.to_string_lossy().into_owned(),
                directory
                    .path()
                    .join("missing.ttf")
                    .to_string_lossy()
                    .into_owned(),
            ],
            None,
        );

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

        let result = collect(
            &[
                cff.to_string_lossy().into_owned(),
                truetype.to_string_lossy().into_owned(),
            ],
            None,
        );

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

        let result = collect(&[font.to_string_lossy().into_owned()], None);

        assert_eq!(result.items[0].status, ItemStatus::Skipped);
        assert_eq!(result.items[0].output_bytes, Some(8));
    }

    #[test]
    fn places_outputs_in_selected_directory() {
        let input_directory = tempfile::tempdir().unwrap();
        let output_directory = tempfile::tempdir().unwrap();
        let font = input_directory.path().join("font.ttf");
        fs::write(&font, b"fixture").unwrap();

        let result = collect(
            &[font.to_string_lossy().into_owned()],
            Some(output_directory.path()),
        );

        assert_eq!(
            Path::new(&result.items[0].output_path),
            fs::canonicalize(output_directory.path())
                .unwrap()
                .join("font.woff2")
        );
        assert!(is_valid_output_for(
            &font,
            Path::new(&result.items[0].output_path),
            ConversionKind::TtfToWoff2
        ));
    }

    #[test]
    fn cli_collection_preserves_relative_directories() {
        let input_directory = tempfile::tempdir().unwrap();
        let output_directory = tempfile::tempdir().unwrap();
        let nested = input_directory.path().join("family").join("weight");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("font.ttf"), b"fixture").unwrap();

        let result = collect_for_cli(
            &[input_directory.path().to_string_lossy().into_owned()],
            Some(output_directory.path()),
            FolderConversionMode::FontToWoff2,
            ExistingOutputPolicy::Skip,
        );

        assert_eq!(result.items.len(), 1);
        assert_eq!(
            Path::new(&result.items[0].output_path),
            fs::canonicalize(output_directory.path())
                .unwrap()
                .join("family")
                .join("weight")
                .join("font.woff2")
        );
    }

    #[test]
    fn cli_collection_applies_mode_to_explicit_files() {
        let directory = tempfile::tempdir().unwrap();
        let font = directory.path().join("font.ttf");
        fs::write(&font, b"fixture").unwrap();

        let result = collect_for_cli(
            &[font.to_string_lossy().into_owned()],
            None,
            FolderConversionMode::Woff2ToFont,
            ExistingOutputPolicy::Skip,
        );

        assert!(result.items.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn cli_collection_supports_existing_output_policies() {
        let input_directory = tempfile::tempdir().unwrap();
        let output_directory = tempfile::tempdir().unwrap();
        let font = input_directory.path().join("font.ttf");
        fs::write(&font, b"fixture").unwrap();
        fs::write(output_directory.path().join("font.woff2"), b"existing").unwrap();
        let paths = &[font.to_string_lossy().into_owned()];

        let skipped = collect_for_cli(
            paths,
            Some(output_directory.path()),
            FolderConversionMode::Both,
            ExistingOutputPolicy::Skip,
        );
        assert_eq!(skipped.items[0].status, ItemStatus::Skipped);

        let failed = collect_for_cli(
            paths,
            Some(output_directory.path()),
            FolderConversionMode::Both,
            ExistingOutputPolicy::Error,
        );
        assert_eq!(failed.items[0].status, ItemStatus::Failed);

        let overwrite = collect_for_cli(
            paths,
            Some(output_directory.path()),
            FolderConversionMode::Both,
            ExistingOutputPolicy::Overwrite,
        );
        assert_eq!(overwrite.items[0].status, ItemStatus::Queued);
    }

    #[test]
    fn cli_collection_fails_every_item_with_the_same_output() {
        let first_directory = tempfile::tempdir().unwrap();
        let second_directory = tempfile::tempdir().unwrap();
        let output_directory = tempfile::tempdir().unwrap();
        let first = first_directory.path().join("font.ttf");
        let second = second_directory.path().join("font.ttf");
        fs::write(&first, b"first").unwrap();
        fs::write(&second, b"second").unwrap();

        let result = collect_for_cli(
            &[
                first.to_string_lossy().into_owned(),
                second.to_string_lossy().into_owned(),
            ],
            Some(output_directory.path()),
            FolderConversionMode::FontToWoff2,
            ExistingOutputPolicy::Overwrite,
        );

        assert_eq!(result.items.len(), 2);
        assert!(
            result
                .items
                .iter()
                .all(|item| item.status == ItemStatus::Failed)
        );
    }

    #[test]
    fn cli_collection_does_not_rescan_nested_output_directory() {
        let input_directory = tempfile::tempdir().unwrap();
        let output_directory = input_directory.path().join("converted");
        fs::create_dir(&output_directory).unwrap();
        fs::write(input_directory.path().join("source.ttf"), b"fixture").unwrap();
        fs::write(output_directory.join("old.ttf"), b"fixture").unwrap();

        let result = collect_for_cli(
            &[input_directory.path().to_string_lossy().into_owned()],
            Some(&output_directory),
            FolderConversionMode::FontToWoff2,
            ExistingOutputPolicy::Skip,
        );

        assert_eq!(result.items.len(), 1);
        assert!(result.items[0].input_path.ends_with("source.ttf"));
    }

    #[test]
    fn marks_duplicate_custom_output_names_as_conflicts() {
        let first_directory = tempfile::tempdir().unwrap();
        let second_directory = tempfile::tempdir().unwrap();
        let output_directory = tempfile::tempdir().unwrap();
        let first = first_directory.path().join("font.ttf");
        let second = second_directory.path().join("font.ttf");
        fs::write(&first, b"first").unwrap();
        fs::write(&second, b"second").unwrap();

        let result = collect(
            &[
                first.to_string_lossy().into_owned(),
                second.to_string_lossy().into_owned(),
            ],
            Some(output_directory.path()),
        );

        assert_eq!(result.items.len(), 2);
        assert_eq!(
            result
                .items
                .iter()
                .filter(|item| item.status == ItemStatus::Skipped)
                .count(),
            1
        );
        assert!(result.items.iter().any(|item| {
            item.message.as_deref() == Some("Output path conflicts with another queued font")
        }));
    }
}

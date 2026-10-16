use crate::{
    Error, ErrorCode, Existing, Format, Item, Mode, Options, Plan, Status, Warning, identify,
};
use std::{
    collections::HashSet,
    fs,
    path::{Component, Path, PathBuf},
};
fn key(path: &Path) -> String {
    let text = path.to_string_lossy().to_string();
    if cfg!(windows) {
        text.to_lowercase()
    } else {
        text
    }
}
fn absolute(path: &Path) -> PathBuf {
    if let Ok(path) = fs::canonicalize(path) {
        return path;
    }
    let joined = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir().unwrap().join(path)
    };
    let mut clean = PathBuf::new();
    for part in joined.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                clean.pop();
            }
            _ => clean.push(part),
        }
    }
    if let (Some(parent), Some(name)) = (clean.parent(), clean.file_name()) {
        return absolute_parent(parent).join(name);
    }
    clean
}
fn absolute_parent(path: &Path) -> PathBuf {
    if let Ok(path) = fs::canonicalize(path) {
        return path;
    }
    if let (Some(parent), Some(name)) = (path.parent(), path.file_name()) {
        return absolute_parent(parent).join(name);
    }
    path.to_owned()
}
fn warning(plan: &mut Plan, path: &Path, error: Error) {
    plan.warnings.push(Warning {
        path: path.to_owned(),
        error_code: error.code,
        message: error.message,
    });
}
pub fn scan(paths: &[PathBuf], options: &Options) -> Plan {
    let mut plan = Plan::default();
    let mut seen = HashSet::new();
    for path in paths {
        if !path.exists() {
            warning(
                &mut plan,
                path,
                Error::new(ErrorCode::InputNotFound, "Input does not exist"),
            );
            continue;
        }
        if path.is_dir() {
            for entry in walkdir::WalkDir::new(path)
                .follow_links(false)
                .follow_root_links(false)
                .sort_by_file_name()
            {
                match entry {
                    Ok(entry) if entry.file_type().is_file() => {
                        let ext = entry
                            .path()
                            .extension()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_ascii_lowercase();
                        if ["ttf", "otf", "woff2", "woff", "ttc", "otc"].contains(&ext.as_str()) {
                            add(&mut plan, &mut seen, entry.path(), Some(path), options);
                        }
                    }
                    Err(error) => warning(
                        &mut plan,
                        error.path().unwrap_or(path),
                        Error::new(ErrorCode::InputUnreadable, error.to_string()),
                    ),
                    _ => {}
                }
            }
        } else {
            add(&mut plan, &mut seen, path, None, options);
        }
    }
    let inputs = seen;
    let mut outputs = std::collections::HashMap::<String, Vec<usize>>::new();
    for (index, item) in plan.items.iter().enumerate() {
        outputs
            .entry(key(&absolute(&item.output)))
            .or_default()
            .push(index);
    }
    for (output, indices) in outputs {
        if indices.len() > 1 || inputs.contains(&output) {
            for index in indices {
                plan.items[index].fail(Error::new(
                    ErrorCode::OutputConflict,
                    "Multiple inputs share this output, or output replaces an input",
                ));
            }
        }
    }
    plan
}
fn add(
    plan: &mut Plan,
    seen: &mut HashSet<String>,
    path: &Path,
    root: Option<&Path>,
    options: &Options,
) {
    let input = match fs::canonicalize(path) {
        Ok(path) => path,
        Err(e) => {
            warning(
                plan,
                path,
                Error::new(ErrorCode::InputUnreadable, e.to_string()),
            );
            return;
        }
    };
    if !seen.insert(key(&input)) {
        return;
    }
    let (input_format, output_format) = match identify(&input) {
        Ok(result) => result,
        Err(error) => {
            warning(plan, path, error);
            return;
        }
    };
    if (options.mode == Mode::Encode && input_format == Format::Woff2)
        || (options.mode == Mode::Decode && input_format != Format::Woff2)
    {
        return;
    }
    let output = if let Some(directory) = &options.output_dir {
        let relative = root
            .and_then(|root| path.strip_prefix(root).ok())
            .unwrap_or_else(|| Path::new(path.file_name().unwrap()));
        absolute(
            &directory
                .join(relative)
                .with_extension(output_format.extension()),
        )
    } else {
        input.with_extension(output_format.extension())
    };
    let bytes = match fs::metadata(&input) {
        Ok(meta) => meta.len(),
        Err(e) => {
            warning(
                plan,
                path,
                Error::new(ErrorCode::InputUnreadable, e.to_string()),
            );
            return;
        }
    };
    let mut item = Item {
        id: plan.items.len(),
        input,
        output,
        input_format,
        output_format,
        status: Status::Queued,
        input_bytes: bytes,
        output_bytes: None,
        error_code: None,
        message: None,
    };
    if bytes > crate::codec::MAX_INPUT {
        item.fail(Error::new(
            ErrorCode::InputTooLarge,
            "Input exceeds 256 MiB",
        ));
    } else if item.output.exists() {
        match options.existing {
            Existing::Skip => {
                item.status = Status::Skipped;
                item.message = Some("Output already exists".into());
            }
            Existing::Error => {
                item.fail(Error::new(ErrorCode::OutputExists, "Output already exists"))
            }
            Existing::Overwrite => {}
        }
    }
    plan.items.push(item);
}

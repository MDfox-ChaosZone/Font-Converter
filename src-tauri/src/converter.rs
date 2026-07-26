use std::{
    fs,
    io::{self, Write},
    path::Path,
};

use tempfile::Builder;

#[derive(Debug)]
pub struct ConversionOutput {
    pub input_bytes: u64,
    pub output_bytes: u64,
}

#[derive(Debug)]
pub enum ConversionError {
    AlreadyExists,
    Failed(String),
}

/// The only module that knows about the upstream encoder API.
pub fn convert(input: &Path, output: &Path) -> Result<ConversionOutput, ConversionError> {
    if output.exists() {
        return Err(ConversionError::AlreadyExists);
    }

    let input_data = fs::read(input).map_err(failed)?;
    let encoded = ttf2woff2::encode(&input_data, ttf2woff2::BrotliQuality::from(11))
        .map_err(|error| ConversionError::Failed(error.to_string()))?;
    let parent = output
        .parent()
        .ok_or_else(|| ConversionError::Failed("Output path has no parent directory".into()))?;

    let mut temporary = Builder::new()
        .prefix(".ttf2woff2-gui-")
        .suffix(".tmp")
        .tempfile_in(parent)
        .map_err(failed)?;
    temporary.write_all(&encoded).map_err(failed)?;
    temporary.flush().map_err(failed)?;
    temporary.as_file().sync_all().map_err(failed)?;

    match temporary.persist_noclobber(output) {
        Ok(file) => {
            file.sync_all().map_err(failed)?;
            Ok(ConversionOutput {
                input_bytes: input_data.len() as u64,
                output_bytes: encoded.len() as u64,
            })
        }
        Err(_error) if output.exists() => Err(ConversionError::AlreadyExists),
        Err(error) => Err(ConversionError::Failed(error.error.to_string())),
    }
}

fn failed(error: io::Error) -> ConversionError {
    ConversionError::Failed(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_output_is_never_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("font.ttf");
        let output = directory.path().join("font.woff2");
        fs::write(&input, b"not needed").unwrap();
        fs::write(&output, b"keep me").unwrap();

        assert!(matches!(
            convert(&input, &output),
            Err(ConversionError::AlreadyExists)
        ));
        assert_eq!(fs::read(output).unwrap(), b"keep me");
    }

    #[test]
    fn invalid_font_does_not_leave_output_or_temp_file() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("broken.ttf");
        let output = directory.path().join("broken.woff2");
        fs::write(&input, b"not a TrueType font").unwrap();

        assert!(matches!(
            convert(&input, &output),
            Err(ConversionError::Failed(_))
        ));
        assert!(!output.exists());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn real_font_fixture_is_deterministic_when_configured() {
        let Some(font) = std::env::var_os("TTF2WOFF2_TEST_FONT") else {
            return;
        };
        let directory = tempfile::tempdir().unwrap();
        let first = directory.path().join("first.woff2");
        let second = directory.path().join("second.woff2");

        convert(Path::new(&font), &first).unwrap();
        convert(Path::new(&font), &second).unwrap();

        let first_bytes = fs::read(first).unwrap();
        assert_eq!(&first_bytes[..4], b"wOF2");
        assert_eq!(first_bytes, fs::read(second).unwrap());
    }
}

use std::{
    fs,
    io::{self, Write},
    path::Path,
};

use tempfile::Builder;
use ttf2woff2_gui_shared::ConversionKind;

const BROTLI_QUALITY: i32 = 11;
const ALLOW_TRANSFORMS: bool = true;
const MAX_DECOMPRESSED_BYTES: usize = 128 * 1024 * 1024;

unsafe extern "C" {
    fn ttf2woff2_google_max_compressed_size(input: *const u8, input_length: usize) -> usize;
    fn ttf2woff2_google_convert(
        input: *const u8,
        input_length: usize,
        output: *mut u8,
        output_length: *mut usize,
        brotli_quality: i32,
        allow_transforms: i32,
    ) -> i32;
    fn ttf2woff2_google_decompressed_size(input: *const u8, input_length: usize) -> usize;
    fn ttf2woff2_google_decompress(
        input: *const u8,
        input_length: usize,
        output: *mut u8,
        output_capacity: usize,
        output_length: *mut usize,
    ) -> i32;
}

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

/// The only Rust module that knows about the Google WOFF2 encoder ABI.
pub fn convert(
    input: &Path,
    output: &Path,
    conversion: ConversionKind,
) -> Result<ConversionOutput, ConversionError> {
    if output.exists() {
        return Err(ConversionError::AlreadyExists);
    }

    let input_data = fs::read(input).map_err(failed)?;
    let converted = match conversion {
        ConversionKind::TtfToWoff2 | ConversionKind::OtfToWoff2 => encode(&input_data)?,
        ConversionKind::Woff2ToTtf | ConversionKind::Woff2ToOtf => decode(&input_data)?,
    };
    let parent = output
        .parent()
        .ok_or_else(|| ConversionError::Failed("Output path has no parent directory".into()))?;

    let mut temporary = Builder::new()
        .prefix(".ttf2woff2-gui-")
        .suffix(".tmp")
        .tempfile_in(parent)
        .map_err(failed)?;
    temporary.write_all(&converted).map_err(failed)?;
    temporary.flush().map_err(failed)?;
    temporary.as_file().sync_all().map_err(failed)?;

    match temporary.persist_noclobber(output) {
        Ok(file) => {
            file.sync_all().map_err(failed)?;
            Ok(ConversionOutput {
                input_bytes: input_data.len() as u64,
                output_bytes: converted.len() as u64,
            })
        }
        Err(_error) if output.exists() => Err(ConversionError::AlreadyExists),
        Err(error) => Err(ConversionError::Failed(error.error.to_string())),
    }
}

fn decode(input: &[u8]) -> Result<Vec<u8>, ConversionError> {
    if input.len() < 8 || &input[..4] != b"wOF2" {
        return Err(ConversionError::Failed(
            "Input does not contain a valid WOFF2 header".into(),
        ));
    }

    // SAFETY: `input` remains alive for the call and exposes exactly `input.len()` bytes.
    let capacity = unsafe { ttf2woff2_google_decompressed_size(input.as_ptr(), input.len()) };
    if capacity == 0 || capacity > MAX_DECOMPRESSED_BYTES {
        return Err(ConversionError::Failed(
            "The WOFF2 output size is invalid or exceeds the 128 MB safety limit".into(),
        ));
    }

    let mut output = vec![0_u8; capacity];
    let mut output_length = 0;
    // SAFETY: the input and output buffers remain alive, the writable capacity is
    // supplied exactly, and the C++ wrapper catches exceptions at the ABI boundary.
    let succeeded = unsafe {
        ttf2woff2_google_decompress(
            input.as_ptr(),
            input.len(),
            output.as_mut_ptr(),
            capacity,
            &mut output_length,
        )
    };
    if succeeded == 0 || output_length == 0 || output_length > capacity {
        return Err(ConversionError::Failed(
            "Google WOFF2 rejected the file or failed to decompress it".into(),
        ));
    }

    output.truncate(output_length);
    Ok(output)
}

fn encode(input: &[u8]) -> Result<Vec<u8>, ConversionError> {
    if input.is_empty() {
        return Err(ConversionError::Failed("Input font is empty".into()));
    }

    // SAFETY: `input` remains alive for the call and exposes exactly `input.len()` bytes.
    let capacity = unsafe { ttf2woff2_google_max_compressed_size(input.as_ptr(), input.len()) };
    if capacity == 0 || capacity > isize::MAX as usize {
        return Err(ConversionError::Failed(
            "Google WOFF2 could not determine a safe output size".into(),
        ));
    }

    let mut output = vec![0_u8; capacity];
    let mut output_length = capacity;
    // SAFETY: both buffers remain alive for the call, `output` has `capacity` writable
    // bytes, and the C++ wrapper catches exceptions before they can cross the ABI.
    let succeeded = unsafe {
        ttf2woff2_google_convert(
            input.as_ptr(),
            input.len(),
            output.as_mut_ptr(),
            &mut output_length,
            BROTLI_QUALITY,
            i32::from(ALLOW_TRANSFORMS),
        )
    };
    if succeeded == 0 || output_length > capacity {
        return Err(ConversionError::Failed(
            "Google WOFF2 rejected the font or failed to encode it".into(),
        ));
    }

    output.truncate(output_length);
    Ok(output)
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
            convert(&input, &output, ConversionKind::TtfToWoff2),
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
            convert(&input, &output, ConversionKind::TtfToWoff2),
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

        convert(Path::new(&font), &first, ConversionKind::TtfToWoff2).unwrap();
        convert(Path::new(&font), &second, ConversionKind::TtfToWoff2).unwrap();

        let first_bytes = fs::read(first).unwrap();
        assert_eq!(&first_bytes[..4], b"wOF2");
        assert_eq!(first_bytes, fs::read(second).unwrap());
    }

    #[test]
    fn real_font_round_trips_when_configured() {
        let Some(font) = std::env::var_os("TTF2WOFF2_TEST_FONT") else {
            return;
        };
        let original = fs::read(&font).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let compressed = directory.path().join("font.woff2");
        let restored = directory.path().join("font.ttf");

        convert(Path::new(&font), &compressed, ConversionKind::TtfToWoff2).unwrap();
        convert(&compressed, &restored, ConversionKind::Woff2ToTtf).unwrap();

        let restored = fs::read(restored).unwrap();
        assert_eq!(&restored[..4], &original[..4]);
        assert!(restored.len() > 12);
    }

    #[test]
    fn real_otf_round_trips_when_configured() {
        let Some(font) = std::env::var_os("TTF2WOFF2_TEST_OTF") else {
            return;
        };
        let original = fs::read(&font).unwrap();
        assert_eq!(&original[..4], b"OTTO");
        let directory = tempfile::tempdir().unwrap();
        let compressed = directory.path().join("font.woff2");
        let restored = directory.path().join("font.otf");

        convert(Path::new(&font), &compressed, ConversionKind::OtfToWoff2).unwrap();
        assert_eq!(&fs::read(&compressed).unwrap()[..8], b"wOF2OTTO");

        convert(&compressed, &restored, ConversionKind::Woff2ToOtf).unwrap();
        assert_eq!(&fs::read(restored).unwrap()[..4], b"OTTO");
    }
}

use crate::{Error, ErrorCode, Existing, Format, Item};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};
pub const MAX_INPUT: u64 = 256 * 1024 * 1024;
pub const MAX_DECODE: usize = 128 * 1024 * 1024;
unsafe extern "C" {
    fn fc_capacity(data: *const u8, length: usize, encode: i32) -> usize;
    fn fc_convert(
        data: *const u8,
        length: usize,
        output: *mut u8,
        capacity: usize,
        written: *mut usize,
        encode: i32,
    ) -> i32;
}
pub fn identify(path: &Path) -> Result<(Format, Format), Error> {
    let mut file =
        fs::File::open(path).map_err(|e| Error::new(ErrorCode::InputUnreadable, e.to_string()))?;
    let mut header = [0u8; 48];
    let size = file
        .read(&mut header)
        .map_err(|e| Error::new(ErrorCode::InputUnreadable, e.to_string()))?;
    if size < 12 {
        return Err(Error::new(ErrorCode::InvalidFont, "Truncated font header"));
    }
    match &header[..4] {
        b"\0\x01\0\0" | b"true" => Ok((Format::Ttf, Format::Woff2)),
        b"OTTO" => Ok((Format::Otf, Format::Woff2)),
        b"wOF2" if size >= 48 => match &header[4..8] {
            b"\0\x01\0\0" | b"true" => Ok((Format::Woff2, Format::Ttf)),
            b"OTTO" => Ok((Format::Woff2, Format::Otf)),
            _ => Err(Error::new(
                ErrorCode::UnsupportedFormat,
                "Unsupported WOFF2 flavor (font collections are not supported)",
            )),
        },
        b"ttcf" | b"wOFF" => Err(Error::new(
            ErrorCode::UnsupportedFormat,
            "Font collections and WOFF1 are not supported",
        )),
        _ => Err(Error::new(
            ErrorCode::InvalidFont,
            "Unrecognized font header",
        )),
    }
}
pub fn convert(item: &Item, existing: Existing) -> Result<u64, Error> {
    let source = fs::canonicalize(&item.input)
        .map_err(|e| Error::new(ErrorCode::InputUnreadable, e.to_string()))?;
    if item.output.exists() {
        if fs::canonicalize(&item.output).ok().as_ref() == Some(&source) {
            return Err(Error::new(
                ErrorCode::OutputConflict,
                "Output would replace an input file",
            ));
        }
        if existing != Existing::Overwrite {
            return Err(Error::new(ErrorCode::OutputExists, "Output already exists"));
        }
    }
    if identify(&source)? != (item.input_format, item.output_format) {
        return Err(Error::new(
            ErrorCode::InvalidFont,
            "Input format changed since scan",
        ));
    }
    let mut file = fs::File::open(source)
        .map_err(|e| Error::new(ErrorCode::InputUnreadable, e.to_string()))?;
    if file
        .metadata()
        .map_err(|e| Error::new(ErrorCode::InputUnreadable, e.to_string()))?
        .len()
        > MAX_INPUT
    {
        return Err(Error::new(
            ErrorCode::InputTooLarge,
            "Input exceeds 256 MiB",
        ));
    }
    let mut data = Vec::new();
    (&mut file)
        .take(MAX_INPUT + 1)
        .read_to_end(&mut data)
        .map_err(|e| Error::new(ErrorCode::InputUnreadable, e.to_string()))?;
    if data.len() as u64 > MAX_INPUT {
        return Err(Error::new(
            ErrorCode::InputTooLarge,
            "Input exceeds 256 MiB",
        ));
    }
    let encode = i32::from(item.input_format != Format::Woff2);
    // SAFETY: data is immutable and alive for the native call; no pointer is retained.
    let capacity = unsafe { fc_capacity(data.as_ptr(), data.len(), encode) };
    let maximum = if encode == 1 {
        MAX_INPUT as usize * 2
    } else {
        MAX_DECODE
    };
    if capacity == 0 || capacity > maximum {
        return Err(Error::new(
            ErrorCode::InputTooLarge,
            "Invalid or excessive codec output size",
        ));
    }
    let mut output = vec![0u8; capacity];
    let mut written = 0usize;
    // SAFETY: buffers are disjoint and sized as passed; C++ exceptions are caught by the wrapper.
    let ok = unsafe {
        fc_convert(
            data.as_ptr(),
            data.len(),
            output.as_mut_ptr(),
            capacity,
            &mut written,
            encode,
        )
    };
    if ok == 0 || written == 0 || written > capacity {
        return Err(Error::new(
            ErrorCode::ConversionFailed,
            "Google WOFF2 rejected the font",
        ));
    }
    output.truncate(written);
    let parent = item
        .output
        .parent()
        .ok_or_else(|| Error::new(ErrorCode::OutputUnwritable, "Missing output directory"))?;
    let io_error = |e: std::io::Error| Error::new(ErrorCode::OutputUnwritable, e.to_string());
    fs::create_dir_all(parent).map_err(io_error)?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".font-converter-")
        .tempfile_in(parent)
        .map_err(io_error)?;
    temporary.write_all(&output).map_err(io_error)?;
    temporary.as_file().sync_all().map_err(io_error)?;
    let persisted = if existing == Existing::Overwrite {
        temporary.persist(&item.output)
    } else {
        temporary.persist_noclobber(&item.output)
    };
    persisted.map_err(|e| {
        if item.output.exists() && existing != Existing::Overwrite {
            Error::new(ErrorCode::OutputExists, "Output appeared during conversion")
        } else {
            io_error(e.error)
        }
    })?;
    Ok(written as u64)
}

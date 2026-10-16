use font_converter_engine::*;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
fn fixture(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures")
        .join(name);
    assert!(
        path.is_file(),
        "Required test fixture missing: {}",
        path.display()
    );
    path
}
fn execute(plan: Plan, options: Options) -> Vec<Item> {
    let batch = start(plan, options, Arc::new(AtomicBool::new(false))).unwrap();
    for event in batch.events {
        if let Event::Finished { items, .. } = event {
            return items;
        }
    }
    panic!("Batch did not finish")
}
fn font_copy(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    fs::copy(fixture("Abel-Regular.ttf"), &path).unwrap();
    path
}
#[derive(Default, Debug, PartialEq)]
struct Outline(Vec<(u8, [f32; 6])>);
impl ttf_parser::OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push((0, [x, y, 0., 0., 0., 0.]));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push((1, [x, y, 0., 0., 0., 0.]));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.push((2, [x1, y1, x, y, 0., 0.]));
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.push((3, [x1, y1, x2, y2, x, y]));
    }
    fn close(&mut self) {
        self.0.push((4, [0.; 6]));
    }
}
#[test]
fn real_ttf_and_otf_roundtrip_semantics() {
    for name in ["Abel-Regular.ttf", "SourceCodePro-Regular.otf"] {
        let temp = tempfile::tempdir().unwrap();
        let source = fixture(name);
        let options = Options {
            output_dir: Some(temp.path().join("encoded")),
            ..Options::default()
        };
        let encoded = execute(scan(std::slice::from_ref(&source), &options), options);
        assert_eq!(encoded[0].status, Status::Succeeded);
        let compressed = fs::read(&encoded[0].output).unwrap();
        assert_eq!(&compressed[..4], b"wOF2");
        let options = Options {
            output_dir: Some(temp.path().join("decoded")),
            ..Options::default()
        };
        let decoded = execute(scan(&[encoded[0].output.clone()], &options), options);
        assert_eq!(decoded[0].status, Status::Succeeded);
        let original = fs::read(source).unwrap();
        let restored = fs::read(&decoded[0].output).unwrap();
        let a = ttf_parser::Face::parse(&original, 0).unwrap();
        let b = ttf_parser::Face::parse(&restored, 0).unwrap();
        assert_eq!(a.number_of_glyphs(), b.number_of_glyphs());
        assert_eq!(a.units_per_em(), b.units_per_em());
        for glyph in 0..a.number_of_glyphs() {
            let id = ttf_parser::GlyphId(glyph);
            assert_eq!(a.glyph_hor_advance(id), b.glyph_hor_advance(id));
            assert_eq!(a.glyph_bounding_box(id), b.glyph_bounding_box(id));
            let mut original_outline = Outline::default();
            let mut restored_outline = Outline::default();
            a.outline_glyph(id, &mut original_outline);
            b.outline_glyph(id, &mut restored_outline);
            assert_eq!(
                original_outline, restored_outline,
                "Glyph {glyph} outline changed"
            );
        }
        for ch in ['A', 'a', '0', '?', ' '] {
            assert_eq!(a.glyph_index(ch), b.glyph_index(ch));
        }
        assert_eq!(
            a.raw_face().table(ttf_parser::Tag::from_bytes(b"name")),
            b.raw_face().table(ttf_parser::Tag::from_bytes(b"name"))
        );
        assert_eq!(
            decoded[0].output_format,
            if name.ends_with("otf") {
                Format::Otf
            } else {
                Format::Ttf
            }
        );
    }
}
#[test]
fn nested_unicode_duplicate_paths_and_dry_scan() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("字体");
    fs::create_dir_all(root.join("子目录")).unwrap();
    let file = font_copy(&root.join("子目录"), "测试.ttf");
    let output = temp.path().join("missing-output");
    let options = Options {
        output_dir: Some(output.clone()),
        ..Options::default()
    };
    let plan = scan(&[root.clone(), file], &options);
    assert_eq!(plan.items.len(), 1);
    assert!(plan.items[0].output.ends_with("子目录/测试.woff2"));
    assert!(!output.exists());
    assert_eq!(execute(plan, options)[0].status, Status::Succeeded);
}
#[test]
fn conflict_marks_every_competing_input() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("a")).unwrap();
    fs::create_dir(temp.path().join("b")).unwrap();
    let paths = [
        font_copy(&temp.path().join("a"), "same.ttf"),
        font_copy(&temp.path().join("b"), "same.ttf"),
    ];
    let options = Options {
        output_dir: Some(temp.path().join("out")),
        ..Options::default()
    };
    let plan = scan(&paths, &options);
    assert_eq!(plan.items.len(), 2);
    assert!(
        plan.items
            .iter()
            .all(|item| item.error_code == Some(ErrorCode::OutputConflict))
    );
    assert!(!temp.path().join("out").exists());
}
#[test]
fn existing_policies_and_atomic_failure() {
    let temp = tempfile::tempdir().unwrap();
    let input = font_copy(temp.path(), "font.ttf");
    let output = input.with_extension("woff2");
    fs::write(&output, b"keep").unwrap();
    assert_eq!(
        scan(std::slice::from_ref(&input), &Options::default()).items[0].status,
        Status::Skipped
    );
    let options = Options {
        existing: Existing::Error,
        ..Options::default()
    };
    assert_eq!(
        scan(std::slice::from_ref(&input), &options).items[0].error_code,
        Some(ErrorCode::OutputExists)
    );
    let options = Options {
        existing: Existing::Overwrite,
        ..Options::default()
    };
    assert_eq!(
        execute(scan(std::slice::from_ref(&input), &options), options)[0].status,
        Status::Succeeded
    );
    let mut bad = [0u8; 12];
    bad[..4].copy_from_slice(b"\0\x01\0\0");
    fs::write(&input, bad).unwrap();
    fs::write(&output, b"keep").unwrap();
    let options = Options {
        existing: Existing::Overwrite,
        ..Options::default()
    };
    let result = execute(scan(&[input], &options), options);
    assert_eq!(result[0].status, Status::Failed);
    assert_eq!(fs::read(&output).unwrap(), b"keep");
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 2);
}
#[test]
fn reject_missing_invalid_collection_and_large_font() {
    let temp = tempfile::tempdir().unwrap();
    let invalid = temp.path().join("bad.ttf");
    fs::write(&invalid, b"broken").unwrap();
    let collection = temp.path().join("collection.ttc");
    fs::write(&collection, b"ttcf12345678").unwrap();
    let large = font_copy(temp.path(), "huge.ttf");
    fs::OpenOptions::new()
        .write(true)
        .open(&large)
        .unwrap()
        .set_len(256 * 1024 * 1024 + 1)
        .unwrap();
    let plan = scan(
        &[temp.path().join("missing"), invalid, collection, large],
        &Options::default(),
    );
    assert_eq!(plan.warnings.len(), 3);
    assert_eq!(plan.items[0].error_code, Some(ErrorCode::InputTooLarge));
}
#[test]
fn unwritable_parent_reports_failure_without_partial_output() {
    let temp = tempfile::tempdir().unwrap();
    let input = font_copy(temp.path(), "font.ttf");
    let blocked = temp.path().join("blocked");
    fs::write(&blocked, b"file").unwrap();
    let options = Options {
        output_dir: Some(blocked),
        ..Options::default()
    };
    let result = execute(scan(&[input], &options), options);
    assert_eq!(result[0].error_code, Some(ErrorCode::OutputUnwritable));
}
#[test]
fn cancel_before_dispatch_and_batch_ids_are_unique() {
    let temp = tempfile::tempdir().unwrap();
    let input = font_copy(temp.path(), "font.ttf");
    let options = Options::default();
    let plan = scan(&[input], &options);
    let cancelled = Arc::new(AtomicBool::new(true));
    let batch = start(plan.clone(), options.clone(), cancelled).unwrap();
    let first_id = batch.id;
    for event in batch.events {
        if let Event::Finished { items, .. } = event {
            assert_eq!(items[0].status, Status::Cancelled);
            assert!(!items[0].output.exists());
        }
    }
    let batch = start(plan, options, Arc::new(AtomicBool::new(false))).unwrap();
    assert_ne!(first_id, batch.id);
    batch.cancel.store(true, Ordering::Release);
    for _ in batch.events {}
}
#[test]
fn cancellation_stops_later_dispatch() {
    let temp = tempfile::tempdir().unwrap();
    let paths: Vec<_> = (0..24)
        .map(|n| font_copy(temp.path(), &format!("font{n}.ttf")))
        .collect();
    let options = Options {
        jobs: 1,
        ..Options::default()
    };
    let batch = start(
        scan(&paths, &options),
        options,
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    for event in batch.events {
        match event {
            Event::Item { item, .. } if item.status == Status::Running => {
                batch.cancel.store(true, Ordering::Release)
            }
            Event::Finished { items, .. } => {
                assert!(items.iter().any(|i| i.status == Status::Cancelled));
                assert!(items.iter().any(|i| i.status == Status::Succeeded));
                assert!(!items.iter().any(|i| i.status == Status::Running));
            }
            _ => {}
        }
    }
}

#[test]
fn encode_cannot_replace_filtered_decode_input() {
    let temp = tempfile::tempdir().unwrap();
    let input = font_copy(temp.path(), "font.ttf");
    let first = execute(
        scan(std::slice::from_ref(&input), &Options::default()),
        Options::default(),
    );
    let compressed = first[0].output.clone();
    let before = fs::read(&compressed).unwrap();
    let options = Options {
        mode: Mode::Encode,
        existing: Existing::Overwrite,
        ..Options::default()
    };
    let plan = scan(&[input, compressed.clone()], &options);
    assert_eq!(plan.items.len(), 1);
    assert_eq!(plan.items[0].error_code, Some(ErrorCode::OutputConflict));
    assert_eq!(fs::read(compressed).unwrap(), before);
}
#[cfg(unix)]
#[test]
fn recursive_scan_does_not_follow_directory_symlinks() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    let other = temp.path().join("outside");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&other).unwrap();
    font_copy(&other, "font.ttf");
    std::os::unix::fs::symlink(&other, root.join("linked")).unwrap();
    assert!(scan(&[root], &Options::default()).items.is_empty());
    assert!(
        scan(&[temp.path().join("outside")], &Options::default())
            .warnings
            .is_empty()
    );
}

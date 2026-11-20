use super::*;
use std::fs;
use std::io::Write;
use std::os::unix::fs::{symlink, PermissionsExt};
use zip::unstable::write::FileOptionsExt;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

/// Build exact entry fixtures independently of the production packing path.
fn fixture(path: &Path, entries: &[(&str, &[u8])]) {
    let mut writer = ZipWriter::new(fs::File::create(path).unwrap());
    for (name, contents) in entries {
        writer
            .start_file(
                *name,
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .unwrap();
        writer.write_all(contents).unwrap();
    }
    writer.finish().unwrap();
}

/// Actual streaming ZIPs preserve Chinese names, empty folders, hidden files, selection and executable bits.
#[test]
fn zip_roundtrip_and_collision_modes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let source = root.join("资料 空格");
    fs::create_dir_all(source.join("空目录")).unwrap();
    fs::write(source.join("中文.txt"), "压缩解压内容").unwrap();
    fs::write(source.join(".hidden"), "hidden").unwrap();
    fs::write(source.join("run.sh"), "#!/bin/sh\n").unwrap();
    fs::set_permissions(source.join("run.sh"), fs::Permissions::from_mode(0o755)).unwrap();
    let single = perform(std::slice::from_ref(&source), Operation::Compress);
    assert!(single.errors.is_empty(), "{:?}", single.errors);
    assert_eq!(single.outputs[0], root.join("资料 空格.zip"));
    let mut archive = ZipArchive::new(fs::File::open(&single.outputs[0]).unwrap()).unwrap();
    assert!(archive.by_name("资料 空格/空目录/").unwrap().is_dir());
    assert_eq!(
        archive
            .by_name("资料 空格/run.sh")
            .unwrap()
            .unix_mode()
            .unwrap()
            & 0o777,
        0o755
    );
    let again = perform(std::slice::from_ref(&source), Operation::Compress);
    assert_eq!(again.outputs[0], root.join("资料 空格 2.zip"));
    let unpacked = perform(&single.outputs, Operation::ExtractHere);
    assert!(unpacked.errors.is_empty(), "{:?}", unpacked.errors);
    assert_eq!(unpacked.outputs[0], root.join("资料 空格 2"));
    assert_eq!(
        fs::read_to_string(root.join("资料 空格 2/中文.txt")).unwrap(),
        "压缩解压内容"
    );
    assert!(root.join("资料 空格 2/空目录").is_dir());
    assert_eq!(
        fs::read_to_string(root.join("资料 空格 2/.hidden")).unwrap(),
        "hidden"
    );
    assert_eq!(
        fs::metadata(root.join("资料 空格 2/run.sh"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o755
    );
    let folder = perform(&single.outputs, Operation::ExtractFolder);
    assert_eq!(folder.outputs[0], root.join("资料 空格 3"));
    assert!(root.join("资料 空格 3/资料 空格/中文.txt").is_file());
    let multiple = perform(
        &[source.clone(), single.outputs[0].clone()],
        Operation::Compress,
    );
    assert_eq!(multiple.outputs[0], root.join("归档.zip"));
    assert_eq!(multiple.succeeded, 1);
    let wrapped = perform(&single.outputs, Operation::Compress);
    assert_eq!(wrapped.outputs[0], root.join("资料 空格.zip.zip"));
    let wrapped_again = perform(&single.outputs, Operation::Compress);
    assert_eq!(wrapped_again.outputs[0], root.join("资料 空格.zip 2.zip"));
    assert!(source.is_dir() && single.outputs[0].is_file());
    assert!(!fs::read_dir(root).unwrap().any(|entry| entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".qyj-")));
}

/// Flat entries land directly in the current directory; colliding folders stay intact rather than merging.
#[test]
fn extracts_flat_files_multiple_archives_and_existing_symlink_names() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let one = root.join("一.ZIP");
    fixture(&one, &[("报告.txt", b"new"), ("folder/nested", b"nested")]);
    fs::write(root.join("报告.txt"), "original").unwrap();
    fs::create_dir(root.join("folder")).unwrap();
    fs::write(root.join("folder/keep"), "keep").unwrap();
    symlink("nonexistent", root.join("报告 2.txt")).unwrap();
    let invalid = root.join("损坏.zip");
    fs::write(&invalid, "not a zip").unwrap();
    let report = perform(&[invalid.clone(), one.clone()], Operation::ExtractHere);
    assert_eq!(report.succeeded, 1);
    assert_eq!(report.errors.len(), 1);
    assert_eq!(
        fs::read_to_string(root.join("报告.txt")).unwrap(),
        "original"
    );
    assert_eq!(fs::read_to_string(root.join("报告 3.txt")).unwrap(), "new");
    assert_eq!(
        fs::read_to_string(root.join("folder/keep")).unwrap(),
        "keep"
    );
    assert!(root.join("folder 2/nested").is_file());
    assert!(!root.join("folder/nested").exists());
    let separate = perform(&[one], Operation::ExtractFolder);
    assert_eq!(separate.outputs[0], root.join("一"));
    assert!(root.join("一/报告.txt").is_file());
    assert!(invalid.exists());
}

/// Unsafe paths, duplicate names and links are rejected before any extracted result is published.
#[test]
fn rejects_unsafe_zip_entries_without_touching_destination() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    for name in [
        "../escape",
        "/tmp/qyj-outside-file",
        "dir/../../escape",
        "ok/../escape",
    ] {
        let file = root.join("unsafe.zip");
        let placeholder = "x".repeat(name.len());
        fixture(
            &file,
            &[("valid-first", b"not published"), (&placeholder, b"attack")],
        );
        let mut bytes = fs::read(&file).unwrap();
        let matches = bytes
            .windows(placeholder.len())
            .enumerate()
            .filter_map(|(offset, window)| (window == placeholder.as_bytes()).then_some(offset))
            .collect::<Vec<_>>();
        assert_eq!(matches.len(), 2);
        for offset in matches {
            bytes[offset..offset + name.len()].copy_from_slice(name.as_bytes());
        }
        fs::write(&file, bytes).unwrap();
        let report = perform(&[file], Operation::ExtractHere);
        assert_eq!(report.succeeded, 0, "{name}");
        assert!(!root.join("valid-first").exists());
    }
    let file = root.join("symlink.zip");
    let mut writer = ZipWriter::new(fs::File::create(&file).unwrap());
    writer
        .add_symlink("link", "../outside", SimpleFileOptions::default())
        .unwrap();
    writer.finish().unwrap();
    assert_eq!(perform(&[file], Operation::ExtractFolder).succeeded, 0);
    assert!(!root.join("symlink").exists());
    assert!(!root.join("link").exists());
    let duplicate = root.join("duplicate.zip");
    // ZIP writers normally reject duplicates; case-insensitive APFS aliases exercise the same no-overwrite rule.
    fixture(&duplicate, &[("same", b"first"), ("SAME", b"second")]);
    let report = perform(&[duplicate], Operation::ExtractHere);
    assert_eq!(report.succeeded, 0);
    assert!(!root.join("same").exists());
}

/// Corrupt data and password-protected archives fail cleanly rather than publishing partial output.
#[test]
fn rejects_bad_crc_password_and_cleans_temporary_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let corrupt = root.join("crc.zip");
    fixture(&corrupt, &[("data", b"unique-fixture-contents")]);
    let mut bytes = fs::read(&corrupt).unwrap();
    let at = bytes
        .windows(b"unique-fixture-contents".len())
        .position(|p| p == b"unique-fixture-contents")
        .unwrap();
    bytes[at] ^= 1;
    fs::write(&corrupt, bytes).unwrap();
    let report = perform(&[corrupt], Operation::ExtractHere);
    assert_eq!(report.succeeded, 0);
    assert!(!root.join("data").exists());
    let encrypted = root.join("password.zip");
    let mut writer = ZipWriter::new(fs::File::create(&encrypted).unwrap());
    writer
        .start_file(
            "secret",
            SimpleFileOptions::default()
                .with_deprecated_encryption(b"test-password")
                .unwrap(),
        )
        .unwrap();
    writer.write_all(b"secret").unwrap();
    writer.finish().unwrap();
    let report = perform(&[encrypted], Operation::ExtractFolder);
    assert_eq!(report.succeeded, 0);
    assert!(report.errors[0].contains("密码"));
    assert!(!root.join("password").exists());
    assert!(!fs::read_dir(root).unwrap().any(|entry| entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".qyj-")));
}

/// Invalid or linked compression inputs never leave a ZIP that looks successfully completed.
#[test]
fn validates_compression_selection_and_cleans_failed_archives() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("file"), "content").unwrap();
    symlink(root.join("file"), root.join("link")).unwrap();
    let report = perform(&[root.join("file"), root.join("link")], Operation::Compress);
    assert_eq!(report.succeeded, 0);
    assert!(!root.join("归档.zip").exists());
    assert_eq!(perform(&[], Operation::Compress).succeeded, 0);
    assert_eq!(
        perform(&[root.join("missing")], Operation::Compress).succeeded,
        0
    );
    let sibling = root.join("sub");
    fs::create_dir(&sibling).unwrap();
    fs::write(sibling.join("file"), "nested").unwrap();
    assert_eq!(
        perform(
            &[root.join("file"), sibling.join("file")],
            Operation::Compress
        )
        .succeeded,
        0
    );
    assert_eq!(
        perform(&[root.join("file")], Operation::ExtractHere).succeeded,
        0
    );
    let empty = root.join("empty.zip");
    ZipWriter::new(fs::File::create(&empty).unwrap())
        .finish()
        .unwrap();
    let report = perform(&[empty], Operation::ExtractFolder);
    assert_eq!(report.succeeded, 1);
    assert!(root.join("empty").is_dir());
    assert!(!fs::read_dir(root).unwrap().any(|entry| entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".qyj-")));
}

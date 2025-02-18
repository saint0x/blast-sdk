use std::fs;
use tempfile::tempdir;
use blast_core::utils::{create_dir_all, copy_dir_all, dir_size, remove_dir_all, hardlink_or_copy};

#[test]
fn test_directory_operations() {
    let temp = tempdir().unwrap();
    let test_dir = temp.path().join("test_dir");

    // Test create_dir_all
    create_dir_all(&test_dir).unwrap();
    assert!(test_dir.exists());

    // Test copy_dir_all
    let src_dir = test_dir.join("src");
    let dst_dir = test_dir.join("dst");
    create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("test.txt"), "test content").unwrap();

    copy_dir_all(&src_dir, &dst_dir).unwrap();
    assert!(dst_dir.exists());
    assert!(dst_dir.join("test.txt").exists());

    // Test dir_size
    let size = dir_size(&test_dir).unwrap();
    assert!(size > 0);

    // Test remove_dir_all
    remove_dir_all(&test_dir).unwrap();
    assert!(!test_dir.exists());
}

#[test]
fn test_hardlink_or_copy() {
    let temp = tempdir().unwrap();
    let src_file = temp.path().join("src.txt");
    let dst_file = temp.path().join("dst.txt");

    fs::write(&src_file, "test content").unwrap();
    hardlink_or_copy(&src_file, &dst_file).unwrap();

    assert!(dst_file.exists());
    assert_eq!(
        fs::read_to_string(&src_file).unwrap(),
        fs::read_to_string(&dst_file).unwrap()
    );
} 
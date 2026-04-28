pub use super::migration_service::{MigrationEvent, migrate_path, move_path_without_progress};

#[cfg(test)]
mod tests {
    use super::{MigrationEvent, migrate_path, move_path_without_progress};
    use crate::domain::error::SymmError;
    use crate::infra::fs::copy_with_progress::copy_path_with_progress;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn move_path_without_progress_moves_file() {
        let temp = tempdir().expect("temp dir");
        let src = temp.path().join("src.txt");
        let dst = temp.path().join("dst.txt");
        fs::write(&src, "payload").expect("write source");
        move_path_without_progress(&src, &dst).expect("move should succeed");
        assert!(!src.exists());
        assert_eq!(fs::read_to_string(&dst).expect("read target"), "payload");
    }

    #[test]
    fn migrate_path_reports_stages_for_same_volume_move() {
        let temp = tempdir().expect("temp dir");
        let src = temp.path().join("src.txt");
        let dst = temp.path().join("dst.txt");
        fs::write(&src, "payload").expect("write source");
        let mut seen = Vec::new();
        migrate_path(&src, &dst, &mut |event| {
            seen.push(event);
            Ok(())
        })
        .expect("move should succeed");
        assert!(matches!(
            seen.first(),
            Some(MigrationEvent::Scanning { .. })
        ));
        assert!(
            seen.iter()
                .any(|event| matches!(event, MigrationEvent::FastMove { .. }))
        );
    }

    #[test]
    fn migrate_path_moves_directory_without_losing_contents() {
        let temp = tempdir().expect("temp dir");
        let src = temp.path().join("src_dir");
        let nested = src.join("nested");
        let dst = temp.path().join("dst_dir");
        fs::create_dir_all(&nested).expect("create nested dir");
        fs::write(nested.join("file.txt"), "payload").expect("write payload");
        move_path_without_progress(&src, &dst).expect("move dir should succeed");
        assert!(!src.exists());
        assert_eq!(
            fs::read_to_string(dst.join("nested").join("file.txt")).expect("read target"),
            "payload"
        );
    }

    #[test]
    fn copy_path_with_progress_cleans_partial_target_when_reporter_aborts() {
        let temp = tempdir().expect("temp dir");
        let src = temp.path().join("src_dir");
        let nested = src.join("nested");
        let dst = temp.path().join("dst_dir");
        fs::create_dir_all(&nested).expect("create nested dir");
        fs::write(nested.join("file.txt"), "payload").expect("write payload");
        let err = copy_path_with_progress(&src, &dst, &mut |_event| {
            Err(SymmError::IoError {
                message: "stop".to_string(),
            })
        })
        .expect_err("reporter abort should stop copy");
        assert!(
            matches!(err, SymmError::IoError { ref message } if message == "stop"),
            "unexpected error: {err:?}"
        );
        assert!(src.exists(), "source should stay in place on abort");
        assert!(
            !dst.exists(),
            "partial destination should be cleaned when copy aborts"
        );
    }
}

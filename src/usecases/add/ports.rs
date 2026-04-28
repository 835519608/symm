use crate::domain::error::SymmError;
use crate::infra::fs::migration::MigrationEvent;
use std::path::Path;

pub trait PathMigrator {
    fn migrate_path<F>(&self, src: &Path, dst: &Path, reporter: &mut F) -> Result<(), SymmError>
    where
        F: FnMut(MigrationEvent) -> Result<(), SymmError>;

    fn move_path_without_progress(&self, src: &Path, dst: &Path) -> Result<(), SymmError>;
}

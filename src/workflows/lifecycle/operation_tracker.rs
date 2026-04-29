use crate::adapters::db::repository;
use crate::domain::error::SymmError;

pub struct OperationTracker<'a> {
    conn: &'a rusqlite::Connection,
    operation_id: String,
}

impl<'a> OperationTracker<'a> {
    pub fn begin(
        conn: &'a rusqlite::Connection,
        command: &str,
        payload: &str,
    ) -> Result<Self, SymmError> {
        let operation_id = repository::begin_operation(conn, command, payload)?;
        Ok(Self { conn, operation_id })
    }

    pub fn pending(&self, step: repository::OperationStep, detail: &str) {
        let _ = repository::advance_operation_step(
            self.conn,
            &self.operation_id,
            step,
            repository::OperationStatus::Pending,
            detail,
        );
    }

    pub fn failed(&self, step: repository::OperationStep, detail: &str) {
        let _ = repository::advance_operation_step(
            self.conn,
            &self.operation_id,
            step,
            repository::OperationStatus::Failed,
            detail,
        );
    }

    pub fn done(&self) {
        let _ = repository::mark_operation_done(self.conn, &self.operation_id);
    }

    pub fn run_pending<T, F>(
        &self,
        step: repository::OperationStep,
        pending_detail: &str,
        failed_prefix: &str,
        op: F,
    ) -> Result<T, SymmError>
    where
        F: FnOnce() -> Result<T, SymmError>,
    {
        self.pending(step, pending_detail);
        match op() {
            Ok(value) => Ok(value),
            Err(err) => {
                self.failed(step, &format!("{failed_prefix}：{err}"));
                Err(err)
            }
        }
    }
}

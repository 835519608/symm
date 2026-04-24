# Symm CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust CLI (`symm`) for managed symlink creation, deletion, and inspection with SQLite-backed registry and cross-platform behavior.

**Architecture:** Implement a small CLI with clear module boundaries: argument parsing, domain models, database access, filesystem link operations, and output rendering. Runtime statuses are derived from filesystem state and not persisted. Windows link creation tries symlink first and falls back to junction for directory targets.

**Tech Stack:** Rust stable, `clap`, `rusqlite` (bundled), `serde`, `serde_json`, `anyhow`/`thiserror`, `assert_cmd`, `tempfile`.

---

### Task 1: Initialize repository and Rust workspace

**Files:**
- Create: `.gitignore`
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/cli.rs`
- Create: `src/model.rs`
- Create: `src/error.rs`
- Create: `src/db.rs`
- Create: `src/link_ops.rs`
- Create: `src/output.rs`
- Create: `tests/cli_flow.rs`

- [ ] **Step 1: Initialize git and Cargo project**

Run:
```bash
git init
cargo init --bin --name symm .
```
Expected:
- `Initialized empty Git repository`
- `Created binary (application) package`

- [ ] **Step 2: Add dependencies to `Cargo.toml`**

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
rusqlite = { version = "0.31", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
anyhow = "1"
dirs = "5"

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
tempfile = "3"
```

- [ ] **Step 3: Add base module wiring**

```rust
// src/main.rs
mod cli;
mod db;
mod error;
mod link_ops;
mod model;
mod output;

fn main() -> anyhow::Result<()> {
    symm::run()
}
```

- [ ] **Step 4: Verify baseline compiles**

Run: `cargo check`
Expected: `Finished` without errors.

- [ ] **Step 5: Commit bootstrap**

Run:
```bash
git add .
git commit -m "chore: bootstrap symm rust cli project"
```

### Task 2: Add failing integration tests for MVP commands

**Files:**
- Modify: `tests/cli_flow.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write failing tests for add/ls/show/rm**

```rust
#[test]
fn add_then_ls_then_show_then_rm() {
    // Use temp dir + SYMM_HOME.
    // 1) add succeeds
    // 2) ls shows name
    // 3) show shows link/target
    // 4) rm succeeds
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test --test cli_flow -- --nocapture`
Expected: FAIL because commands are not implemented.

- [ ] **Step 3: Add minimal command dispatch skeleton**

```rust
enum Commands { Add, Rm, Ls, Show }
```

- [ ] **Step 4: Re-run test and confirm still failing at behavior level**

Run: `cargo test --test cli_flow -- --nocapture`
Expected: FAIL with "not implemented" style error, not parse errors.

- [ ] **Step 5: Commit test scaffold**

Run:
```bash
git add tests/cli_flow.rs src/main.rs src/cli.rs
git commit -m "test: add failing integration tests for mvp commands"
```

### Task 3: Implement data model, DB schema, and basic CRUD

**Files:**
- Modify: `src/model.rs`
- Modify: `src/db.rs`
- Modify: `src/error.rs`
- Test: `tests/cli_flow.rs`

- [ ] **Step 1: Add failing unit tests for DB uniqueness and lookup**

```rust
#[test]
fn unique_name_conflict_maps_to_name_conflict() {}
#[test]
fn unique_link_conflict_maps_to_path_conflict() {}
```

- [ ] **Step 2: Run tests to verify failures**

Run: `cargo test db:: -- --nocapture`
Expected: FAIL for missing schema/logic.

- [ ] **Step 3: Implement schema migration and CRUD**

```sql
CREATE TABLE IF NOT EXISTS links (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  link_path TEXT NOT NULL UNIQUE,
  target_path TEXT NOT NULL,
  link_kind TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
```

- [ ] **Step 4: Re-run tests**

Run: `cargo test`
Expected: DB tests pass; CLI flow still fails on filesystem behavior.

- [ ] **Step 5: Commit DB layer**

Run:
```bash
git add src/model.rs src/db.rs src/error.rs tests/cli_flow.rs
git commit -m "feat: add sqlite registry schema and core crud"
```

### Task 4: Implement link creation/removal with Windows fallback

**Files:**
- Modify: `src/link_ops.rs`
- Modify: `src/model.rs`
- Test: `tests/cli_flow.rs`

- [ ] **Step 1: Write failing tests for runtime status and missing link behavior**

```rust
#[test]
fn status_ok_when_link_and_target_exist() {}
#[test]
fn status_missing_when_registry_exists_but_link_deleted() {}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test --test cli_flow -- --nocapture`
Expected: FAIL with incorrect status/ops behavior.

- [ ] **Step 3: Implement cross-platform operations**

```rust
// Unix: std::os::unix::fs::symlink
// Windows: std::os::windows::fs::{symlink_dir, symlink_file}
// Fallback: junction crate or mklink /J wrapper for directory target
```

- [ ] **Step 4: Re-run tests**

Run: `cargo test --test cli_flow -- --nocapture`
Expected: core lifecycle passes on current OS.

- [ ] **Step 5: Commit filesystem ops**

Run:
```bash
git add src/link_ops.rs src/model.rs tests/cli_flow.rs
git commit -m "feat: implement cross-platform link operations and status checks"
```

### Task 5: Implement command handlers and output modes

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Modify: `src/output.rs`
- Modify: `src/error.rs`
- Test: `tests/cli_flow.rs`

- [ ] **Step 1: Add failing tests for `--json` output shape**

```rust
#[test]
fn ls_json_contains_code_and_fields() {}
#[test]
fn show_json_contains_status_field() {}
```

- [ ] **Step 2: Run tests to verify failures**

Run: `cargo test --test cli_flow -- --nocapture`
Expected: FAIL for missing JSON contract.

- [ ] **Step 3: Implement handlers and formatter**

```rust
symm add <name> <target> <link>
symm rm <name_or_link>
symm ls [--json] [--status ...]
symm show <name_or_link> [--json]
```

- [ ] **Step 4: Re-run full test suite**

Run: `cargo test`
Expected: PASS for unit and integration tests.

- [ ] **Step 5: Commit command layer**

Run:
```bash
git add src/cli.rs src/main.rs src/output.rs src/error.rs tests/cli_flow.rs
git commit -m "feat: implement mvp commands with table and json output"
```

### Task 6: Verification, docs, and release-ready polish

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-24-symm-cli-design.md` (if needed for drift alignment)

- [ ] **Step 1: Add usage docs**

```md
# symm
symm add ...
symm ls --json
```

- [ ] **Step 2: Run quality checks**

Run:
```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```
Expected: all commands succeed.

- [ ] **Step 3: Manual smoke checks**

Run:
```bash
cargo run -- add demo ./target_file ./demo_link
cargo run -- ls
cargo run -- show demo
cargo run -- rm demo
```
Expected: lifecycle works and outputs are correct.

- [ ] **Step 4: Commit final polish**

Run:
```bash
git add README.md
git commit -m "docs: add usage guide and finalize mvp verification"
```

- [ ] **Step 5: Tag optional milestone**

Run: `git tag -a v0.1.0 -m "symm mvp"`
Expected: local tag created.

## Spec Coverage Checklist

- CLI commands add/rm/ls/show: covered in Tasks 2 and 5.
- SQLite registry and indexed lookup behavior: covered in Task 3.
- Runtime status model (`ok|broken|missing`): covered in Task 4.
- Windows symlink fallback to junction: covered in Task 4.
- Table + JSON output: covered in Task 5.
- Reliability and verification: covered in Task 6.


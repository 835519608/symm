# Symm CLI Design (MVP)

## Goal

Build a high-performance, cross-platform symlink management CLI tool that supports create, delete, and view operations.

MVP scope is intentionally narrow:
- CLI-only interface.
- Manage only links created by this tool.
- Windows fallback from symlink to junction for directory targets when symlink creation fails.
- Human-readable output by default, with optional JSON output for automation.

## Non-Goals (MVP)

- Importing external existing symlinks into management.
- TUI interface.
- Batch templates or remote sync.

## Product Decisions (Confirmed)

- Interface: CLI first.
- Managed scope: tool-created links only.
- Language: Rust.
- Windows strategy: prefer symlink, fallback to junction for directory target.
- List/show output: readable table by default, plus `--json`.

## Command Surface

Binary name: `symm`

- `symm add <name> <target> <link>`
  - Create link at `<link>` pointing to `<target>`.
  - Register metadata in SQLite.
- `symm rm <name|link>`
  - Remove by logical name or link path.
  - Deletes link object if present, then removes metadata.
- `symm ls [--json] [--status ok|broken|missing]`
  - List all managed links from registry.
  - Runtime status is computed from filesystem checks.
- `symm show <name|link> [--json]`
  - Show one managed entry with resolved runtime status.

## Storage and Data Model

Default data root:
- `$HOME/.symm/`

Override:
- `SYMM_HOME` environment variable.

Database:
- `<SYMM_HOME>/symm.db`

Table: `links`
- `id` INTEGER PRIMARY KEY
- `name` TEXT NOT NULL UNIQUE
- `link_path` TEXT NOT NULL UNIQUE
- `target_path` TEXT NOT NULL
- `link_kind` TEXT NOT NULL (`symlink` or `junction`)
- `created_at` INTEGER NOT NULL (unix seconds)
- `updated_at` INTEGER NOT NULL (unix seconds)

Indexes:
- Unique index on `name`
- Unique index on `link_path`

## Runtime Status Model

Status is derived at read time and not persisted:

- `ok`: link exists and target exists.
- `broken`: link exists but target does not exist.
- `missing`: registry entry exists but link object does not exist.

This keeps write paths minimal and avoids stale status fields.

## Cross-Platform Link Strategy

Linux/macOS:
- Use native symlink creation.

Windows:
- Try native symlink first.
- If symlink creation fails and target is a directory, fallback to junction creation.
- Persist actual created kind (`symlink` or `junction`) in `link_kind`.

Rationale:
- Maintains compatibility while maximizing successful operation on common Windows setups.

## Consistency and Transaction Rules

`add` flow:
1. Validate args and normalize paths.
2. Create link on filesystem.
3. Insert registry row in DB transaction.
4. If DB insert fails, rollback filesystem by removing just-created link.

`rm` flow:
1. Resolve entry by `name` or `link_path`.
2. Attempt filesystem removal (if already missing, continue).
3. Delete DB row in transaction.
4. Operation is idempotent for missing filesystem link.

All DB writes are transactional.

## Error Contract

Use structured error categories:
- `invalid_argument`
- `permission_denied`
- `target_not_found`
- `name_conflict`
- `path_conflict`
- `db_error`
- `io_error`

For `--json`, return machine-readable payload with:
- `code`
- `message`
- `context` (optional key/value details)

## Performance Considerations

- Registry-first reads: no recursive scans.
- Indexed lookups by `name` and `link_path`.
- Minimal filesystem probes per row for status.
- Keep data model narrow to reduce serialization/IO overhead.

This design favors predictable latency as managed link count grows.

## Implementation Structure

- `src/main.rs`: program entry and command dispatch.
- `src/cli.rs`: clap argument model and subcommands.
- `src/db.rs`: SQLite init/migrations/queries/transactions.
- `src/link_ops.rs`: cross-platform link create/remove behavior.
- `src/model.rs`: domain types and status derivation.
- `src/output.rs`: table and JSON rendering.
- `tests/cli_flow.rs`: integration tests.

## Milestones

M1:
- Project bootstrap.
- Implement `add`, `rm`, `ls`, `show` happy paths.

M2:
- Add transaction safety and conflict mapping.
- Stabilize error categories and JSON error output.

M3:
- Complete Windows symlink->junction fallback logic.
- Add platform-focused integration tests.

M4:
- Output polish, help text, usage docs.
- Final reliability pass.

## Test Plan (MVP)

Unit tests:
- Path normalization.
- Status derivation.
- Conflict classification.

Integration tests:
- Full add/rm/ls/show lifecycle.
- Broken target status behavior.
- Missing link object behavior.
- Windows fallback behavior (platform-gated tests).

Concurrency smoke:
- Parallel add with same name should permit only one success (DB uniqueness guarantee).

## Risks and Mitigations

- Windows privilege/model differences:
  - Mitigation: deterministic fallback path and explicit created-kind recording.
- Incomplete rollback after add failure:
  - Mitigation: rollback helper and integration test coverage.
- Path ambiguity across platforms:
  - Mitigation: canonicalization where possible, normalized persisted path strings.

## Acceptance Criteria

- User can add, remove, list, and inspect managed links via CLI.
- Managed data survives process restart.
- Runtime status reports `ok|broken|missing` accurately.
- Windows directory links succeed through symlink or fallback junction.
- `ls` and `show` support both readable default and `--json`.


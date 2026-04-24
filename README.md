# symm

High-performance cross-platform symlink manager CLI.

## Commands

- `symm add <name> <target> <link>`
- `symm rm <name|link>`
- `symm ls [--status ok|broken|missing] [--json]`
- `symm show <name|link> [--json]`

## Data

- Default home: `$HOME/.symm`
- Override with `SYMM_HOME`
- Registry DB: `symm.db`

## Notes

- Linux/macOS: uses symlink.
- Windows: tries symlink first; directory links fallback to junction.

# Hydra 🐍

A fast duplicate file finder and cleaner written in Rust.

Hydra scans your directory for duplicate files created by copy operations (e.g., `file copy.txt`, `file (1).txt`, `file - Copy.txt`) and safely removes them, keeping only the original.

## Supported Duplicate Patterns

| Source | Pattern | Example |
|--------|---------|---------|
| macOS | `file copy.ext`, `file copy N.ext` | `report copy.pdf`, `report copy 2.pdf` |
| Windows | `file - Copy.ext`, `file - Copy (N).ext` | `photo - Copy.jpg`, `photo - Copy (2).jpg` |
| Browsers | `file (N).ext` | `download (1).zip`, `image (3).png` |

## Installation

```bash
git clone git@github.com:jconvery1/hydra.git
cd hydra
cargo build --release
```

The binary will be available at `target/release/hydra`.

To install globally:

```bash
cargo install --path .
```

## Usage

```bash
# Navigate to the directory you want to clean
cd ~/Downloads

# Preview duplicates (no files deleted)
hydra --dry-run

# Find and delete duplicates (with confirmation prompt)
hydra
```

### Example Output

```
Running in DRY RUN mode - no files will be deleted

--- Duplicate Set ---
Normalized filename: report.pdf
Size: 245832 bytes
Keeping: /Users/you/Downloads/report.pdf
Would delete: /Users/you/Downloads/report copy.pdf
Would delete: /Users/you/Downloads/report copy 2.pdf

================================
Summary: Found 1 duplicate set(s)
Total files to delete: 2

[DRY RUN MODE] No files were deleted.
Run without --dry-run to actually delete files.
```

## License

MIT


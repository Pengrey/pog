# pog

A pentester's log – track, organise and browse security findings from the terminal.

Findings are plain **Markdown files** stored in a well-defined directory
layout. You can use `pog` to import, search and visualise them, **or** just
browse the file tree directly with `find`, `grep`, `cat`, `tree`, etc.

---

## Features

- **Markdown-native** – findings are plain `.md` files you can read, edit and version-control with standard tools.
- **Asset-based organisation** – findings are automatically grouped by target asset with unique hex IDs.
- **Interactive TUI** – tabbed dashboard with keyboard & mouse support.
  - **Graph** – severity distribution bars + a line chart of findings over time (Braille markers), with per-severity toggle filters.
  - **Search** – full-text search across all findings with severity, asset, status and date filters, plus a detail panel.
- **CSV export** – one-command export of the entire database.
- **Bulk import** – import an entire directory of finding folders at once.
- **Zero config** – runs out of the box; data lives in `~/.pog` (or `$POGDIR`).

---

## Quick start

```bash
# Build release binary
make release
# or: cargo build --release

# Import a single finding
pog import -p ./sql-injection

# Bulk-import a directory of findings
pog import -p ./pentest-findings --bulk

# Open the TUI dashboard
pog view

# Export everything to CSV
pog export -o findings.csv

# Wipe the database and all stored findings
pog clean

# Run the test suite
make test
# or: cargo test --workspace
```

---

## Commands

### `pog import`

Import one or more findings into the POGDIR.

```
pog import -p <path>          # single finding folder
pog import -p <path> --bulk   # every sub-folder is a finding
```

Each finding folder must contain **one `.md` file** and may include an
`img/` sub-directory with screenshots.

**Example output:**

```
$ pog import -p ./sql-injection
[+] Imported: SQL Injection [Critical] (nexus_portal)

$ pog import -p ./findings --bulk
[+] Imported 3 finding(s)
[*]   SQL Injection [Critical] (nexus_portal)
[*]   Open Redirect [Medium] (nexus_portal)
[*]   Weak TLS [Low] (orion_gateway)
```

### `pog view`

Launch an interactive TUI with multiple tabs:

| Tab      | Description |
|----------|-------------|
| **Graph**  | Severity distribution bars, a Braille line chart of findings over time, and a severity toggle filter panel. |
| **Search** | Live-search across all findings with severity / asset / status / date filters, scrollable list and a detail panel. |

**Keyboard shortcuts:**

| Key               | Action                          |
|-------------------|---------------------------------|
| `Tab` / `Shift+Tab` | Switch between tabs            |
| `↑` / `k`        | Move cursor up                  |
| `↓` / `j`        | Move cursor down                |
| `Space` / `Enter` | Toggle filter / select item    |
| `/`               | Focus search input (Search tab) |
| `Esc`             | Unfocus / close popup           |
| `q`               | Quit                            |

```
pog view
```

### `pog export`

Export all findings from the database to a CSV file.

```
pog export                    # writes to findings.csv (default)
pog export -o report.csv      # custom output path
```

The CSV contains one row per finding with the columns:
`hex_id`, `title`, `severity`, `asset`, `date`, `location`, `status`, `description`.

### `pog clean`

Delete the database and every stored finding under POGDIR. The empty
directory structure is recreated so you can immediately start fresh.

```
$ pog clean
[+] Database and findings directory wiped clean
```

### `pog report`

*(Planned)* Generate a report from the stored findings.

```
pog report -t template.md -o report.pdf
```

---

## Finding format

Each finding is a folder with the following structure:

```
sql-injection/
├── finding.md
└── img/
    └── proof.png
```

The Markdown file uses a simple metadata + description format:

```markdown
# SQL Injection

- **Severity:** Critical
- **Asset:** Nexus Portal
- **Location:** https://portal.nexus.corp/api/users?id=1
- **Status:** Open
- **Date:** 2026/01/15

## Description

User input is directly concatenated into the SQL query without
sanitisation, allowing an attacker to execute arbitrary commands.
```

| Field        | Required | Default   | Notes                                              |
|--------------|----------|-----------|----------------------------------------------------|
| `# Title`    | yes      | folder name | First level-1 heading                            |
| `Severity`   | no       | `Info`    | `Critical`, `High`, `Medium`, `Low`, `Info`        |
| `Asset`      | no       | `unknown` | Normalised to lowercase with underscores           |
| `Location`   | no       | *(empty)* | URL, file path, etc.                              |
| `Status`     | no       | `Open`    | `Open`, `InProgress`, `Resolved`, `FalsePositive`  |
| `Date`       | no       | *(empty)* | Format: `YYYY/MM/DD`                               |
| `Description`| no       | *(empty)* | Everything under `## Description`                 |

---

## POGDIR – internal file structure

All data lives under a single root directory called **POGDIR**:

| Priority | Source              |
|----------|---------------------|
| 1        | `$POGDIR` env var   |
| 2        | `$HOME/.pog`        |

### Layout

```
~/.pog/                                 # POGDIR root
├── pog.db                              # SQLite database
└── findings/                           # one sub-dir per asset
    ├── nexus_portal/
    │   ├── 0x001_sql-injection/
    │   │   ├── finding.md
    │   │   └── img/
    │   │       └── proof.png
    │   └── 0x002_open-redirect/
    │       └── finding.md
    └── orion_gateway/
        └── 0x001_weak-tls/
            └── finding.md
```

- **Asset folders** group findings by the target under test (always
  lowercase, underscores for spaces).
- Each finding gets a **unique hex ID** (`0x001`, `0x002`, …) scoped to
  its asset, so two findings with similar names never collide.
- The folder name is `<hex_id>_<slug>`, where the slug is the original
  folder name passed to `pog import`.

### Browsing with Unix tools

Because everything is plain files, you don't *need* `pog` to explore your data:

```bash
# List all assets
ls ~/.pog/findings/

# List findings for a specific asset
ls ~/.pog/findings/nexus_portal/

# Read a finding
cat ~/.pog/findings/nexus_portal/0x001_sql-injection/finding.md

# Show the full tree
tree ~/.pog/findings/

# Search all findings for a keyword
grep -rl "SQL" ~/.pog/findings/

# Count findings per asset
find ~/.pog/findings/ -mindepth 2 -maxdepth 2 -type d | \
  awk -F/ '{print $(NF-1)}' | sort | uniq -c | sort -rn

# List all Critical findings
grep -rl "Severity.*Critical" ~/.pog/findings/

# Find all findings with screenshots
find ~/.pog/findings/ -name "img" -type d
```

---

## Building

Requires **Rust 2024 edition** (rustc ≥ 1.85).

```bash
# Release build (recommended)
make release

# Run tests
make test

# Clean build artifacts
make clean
```

Or use `cargo` directly:

```bash
cargo build --release
cargo test --workspace
```

### Project structure

```
pog/
├── src/          # binary entry point
├── cli/          # CLI argument parsing (clap)
├── models/       # domain types – Finding, Severity, GraphData
├── storage/      # POGDIR layout, SQLite, import logic
├── tui/          # ratatui-based terminal UI (tabs: Graph, Search)
├── Makefile
└── Cargo.toml    # workspace root
```

---

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
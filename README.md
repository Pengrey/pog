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
- **PDF reports** – generate professional template-driven PDF reports scoped by asset and date range (requires `pdflatex`, see [Dependencies](#dependencies)).
- **CSV export** – one-command export of the entire database.
- **Bulk import** – import an entire directory of finding folders at once.
- **Zero config** – runs out of the box; data lives in `~/.pog` (or `$POGDIR`).

---

## Dependencies

`pog` itself is a single static binary with no runtime dependencies for
most commands. **PDF report generation** (`pog report`) requires a working
`pdflatex` installation.

### Installing pdflatex

**Arch Linux:**

```bash
sudo pacman -S texlive-basic texlive-bin texlive-latex \
               texlive-latexrecommended texlive-fontsrecommended
```

**Debian / Ubuntu:**

```bash
sudo apt install texlive-latex-base texlive-latex-recommended \
                 texlive-fonts-recommended
```

**Fedora:**

```bash
sudo dnf install texlive-latex texlive-collection-fontsrecommended
```

**macOS (Homebrew):**

```bash
brew install --cask mactex-no-gui
```

Verify the installation with:

```bash
pdflatex --version
```

---

## Quick start

```bash
# Build release binary
make release
# or: cargo build --release

# Import a single finding
pog import -p ./sql-injection

# Bulk-import the example findings
pog import -p ./examples/findings_example --bulk

# Open the TUI dashboard
pog view

# Export everything to CSV
pog export -o findings.csv

# Generate a PDF report
pog report -t examples/report_template_example/template.tmpl \
           --asset nexus_portal --from 2025/09/01 --to 2026/01/31

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
[*] SQL Injection [Critical] (nexus_portal)
[*] Open Redirect [Medium] (nexus_portal)
[*] Weak TLS [Low] (orion_gateway)
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

Generate a PDF report from findings. The report is driven by a
[MiniJinja](https://docs.rs/minijinja) template (Jinja2-compatible) that
uses `#!` directives to describe the PDF structure.

All flags are required:

```
pog report -t template.tmpl --asset nexus_portal --from 2025/09/01 --to 2026/01/31
pog report -t template.tmpl -o report.pdf --asset nexus_portal --from 2025/09/01 --to 2026/01/31
```

| Flag        | Description                                      | Default         |
|-------------|--------------------------------------------------|-----------------|
| `-t`        | Report template file (`.tmpl`)                   | *(required)*    |
| `-o`        | Output PDF path                                  | `report.pdf`    |
| `--asset`   | Asset name to report on                          | *(required)*    |
| `--from`    | Start date (`YYYY/MM/DD`)                        | *(required)*    |
| `--to`      | End date (`YYYY/MM/DD`)                          | *(required)*    |

#### Template directives

Templates are plain text files processed by MiniJinja, then parsed by
`pog` via `#!` directives.  Plain text between directives is rendered as
**Markdown** — bold, italic, inline code, fenced code blocks, headings,
bullet lists, and `[text](url)` links are all supported natively.

| Directive | Description |
|-----------|-------------|
| `#! title <text>` | Large title with accent bar |
| `#! subtitle <text>` | Smaller gray subtitle |
| `#! section <text>` | Section heading with accent underline |
| `#! finding <severity> <text>` | Finding card — auto page-break between findings |
| `#! meta <key>: <value>` | Key–value metadata line |
| `#! table` | Table from following `\|`-delimited lines (first row = header) |
| `#! index` | Auto-generated TOC with dot leaders, page numbers & PDF bookmarks |
| `#! spacer <mm>` | Explicit vertical spacing in millimetres |
| `#! comment <text>` | Template-only note (not rendered in PDF) |
| `#! image <path>` | *(reserved)* Image / logo placeholder |
| `#! pagebreak` | Force a new page |
| `#! hr` | Horizontal rule |

#### Markdown in descriptions

Finding descriptions (and any plain text) support Markdown formatting:

| Syntax | Rendered as |
|--------|-------------|
| `**bold**` | **Bold** text |
| `*italic*` | *Italic* text |
| `***bold italic***` | ***Bold italic*** text |
| `` `code` `` | Inline code with background |
| ` ```…``` ` | Fenced code block with accent bar |
| `[text](url)` | Clickable link (underlined, blue) |
| `# Heading` | Heading (levels 1–3) |
| `- item` | Bullet list item |

#### Template variables

| Variable      | Type   | Description                              |
|---------------|--------|------------------------------------------|
| `findings`    | list   | Array of finding objects (see below)     |
| `date`        | string | Report generation date (`YYYY/MM/DD`)    |
| `asset`       | string | Asset name                               |
| `from`        | string | Start date                               |
| `to`          | string | End date                                 |
| `total`       | int    | Total finding count                      |
| `critical`    | int    | Critical count                           |
| `high`        | int    | High count                               |
| `medium`      | int    | Medium count                             |
| `low`         | int    | Low count                                |
| `info`        | int    | Info count                               |

**Each finding object:**

| Field            | Description                                   |
|------------------|-----------------------------------------------|
| `num`            | 1-based index                                 |
| `title`          | Finding title                                 |
| `severity`       | `Critical`, `High`, `Medium`, `Low`, `Info`   |
| `asset`          | Target asset                                  |
| `date`           | Finding date                                  |
| `location`       | URL or path                                   |
| `description`    | Full description text                         |
| `status`         | `Open`, `In Progress`, `Resolved`, …          |

#### Example snippets

```
{# Cover page with logo placeholder #}
#! title Security Assessment Report
#! comment TODO: Add company logo here once image support is implemented
#! comment       #! image ./assets/company_logo.png
#! subtitle {{ asset }}
#! spacer 8
#! meta Prepared for: {{ asset }}
#! meta Assessment Period: {{ from }} – {{ to }}
#! meta Report Generated: {{ date }}
#! meta Classification: Confidential
#! pagebreak

{# Auto-generated table of contents #}
#! subtitle Table of Contents
#! spacer 4
#! index
#! pagebreak

{# Conditional content #}
{% if critical > 0 %}
ATTENTION: {{ critical }} critical finding(s) require immediate remediation.
{% endif %}

{# Severity summary table with risk levels #}
#! table
Severity | Count | Risk Level
Critical | {{ critical }} | Immediate remediation required
High | {{ high }} | Short-term remediation recommended
Medium | {{ medium }} | Planned remediation advised
Low | {{ low }} | Address during regular maintenance
Info | {{ info }} | Informational / best practice

{# Loop over findings — each starts on its own page #}
{% for f in findings %}
#! finding {{ f.severity }} {{ f.num }}. {{ f.title }}
#! meta Location: {{ f.location }}
{{ f.description }}
{% endfor %}
```

See [`examples/report_template_example/template.tmpl`](examples/report_template_example/template.tmpl) for a complete working template.

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
├── src/                    # binary entry point
├── cli/                    # CLI argument parsing (clap)
├── models/                 # domain types – Finding, Severity, GraphData
├── storage/                # POGDIR layout, SQLite, import & report logic
├── tui/                    # ratatui-based terminal UI (tabs: Graph, Search)
├── examples/
│   ├── findings_example/           # sample finding folders
│   └── report_template_example/    # MiniJinja report template (.tmpl)
├── Makefile
└── Cargo.toml              # workspace root
```

---

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
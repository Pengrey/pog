# pog

A pentester's log – track, organise and browse security findings and assets from the terminal.

Findings and assets are plain **Markdown files**. Use `pog` to import, search and visualise them, **or** browse the file tree with standard Unix tools.

---

## Features

| Feature | Description |
|---------|-------------|
| **Markdown-native** | Plain `.md` files – read, edit and version-control with any tool. |
| **Asset management** | Track assets with metadata: name, description, contact, criticality, DNS/IP. |
| **Asset-based organisation** | Findings grouped by target asset with unique hex IDs. |
| **Interactive TUI** | Tabbed dashboard (Graph, Search, Assets) with keyboard & mouse. |
| **PDF reports** | Template-driven PDF reports scoped by asset and date range. |
| **CSV export** | One-command export of all findings. |
| **Bulk import** | Batch-import findings or assets in one go. |
| **Zero config** | Data lives in `~/.pog` (or `$POGDIR`). |

---

## Quick start

```bash
# Build (Podman container)
make pod-build    # first time only
make release      # compile & strip

# Import findings
pog import-findings -p ./sql-injection
pog import-findings -p ./examples/findings_example --bulk

# Import assets
pog import-assets -p ./asset.md
pog import-assets -p ./assets.md --bulk

# TUI dashboard
pog view

# Export & report
pog export -o findings.csv
pog report -t template.tmpl --asset nexus_portal --from 2025/09/01 --to 2026/01/31

# Wipe everything
pog clean
```

---

## Commands

### `pog import-findings`

Import one or more findings into the POGDIR.

```
pog import-findings -p <path>          # single finding folder
pog import-findings -p <path> --bulk   # every sub-folder is a finding
```

Each finding folder must contain **one `.md` file** and may include an `img/` sub-directory.

```
$ pog import-findings -p ./sql-injection
[+] Imported: SQL Injection [Critical] (nexus_portal)

$ pog import-findings -p ./findings --bulk
[+] Imported 3 finding(s)
```

### `pog import-assets`

Import one or more assets from a Markdown file.

```
pog import-assets -p asset.md            # single asset
pog import-assets -p assets.md --bulk    # multiple assets (--- separated)
```

```
$ pog import-assets -p ./asset.md
[+] Imported asset: nexus_portal [Critical]

$ pog import-assets -p ./assets.md --bulk
[+] Imported 3 asset(s)
```

### `pog view`

Launch an interactive TUI with three tabs:

| Tab | Description |
|-----|-------------|
| **Graph** | Severity distribution bars, Braille line chart, severity toggle filters. |
| **Search** | Full-text search with severity / asset / status filters and detail panel. |
| **Assets** | Scrollable asset list with detail panel showing all metadata. |

**Keyboard shortcuts:**

| Key | Action |
|-----|--------|
| `Tab` / `t` | Switch tab |
| `↑` / `k` | Move up |
| `↓` / `j` | Move down |
| `s` | Focus search (Search tab) |
| `f` | Severity filter (Search tab) |
| `a` | Asset filter (Search tab) |
| `Esc` | Unfocus / close |
| `q` | Quit |

### `pog export`

```
pog export                    # writes to findings.csv
pog export -o report.csv      # custom path
```

### `pog report`

Generate a PDF report from findings using a [MiniJinja](https://docs.rs/minijinja) template.

```
pog report -t template.tmpl --asset nexus_portal --from 2025/09/01 --to 2026/01/31
```

| Flag | Description | Default |
|------|-------------|---------|
| `-t` | Template file (`.tmpl`) | *(required)* |
| `-o` | Output PDF path | `report.pdf` |
| `--asset` | Asset name | *(required)* |
| `--from` | Start date (`YYYY/MM/DD`) | *(required)* |
| `--to` | End date (`YYYY/MM/DD`) | *(required)* |

#### Template directives

| Directive | Description |
|-----------|-------------|
| `#! title <text>` | Large title with accent bar |
| `#! subtitle <text>` | Smaller gray subtitle |
| `#! section <text>` | Section heading |
| `#! finding <severity> <text>` | Finding card (auto page-break) |
| `#! meta <key>: <value>` | Key–value line |
| `#! table` | Table from `\|`-delimited lines |
| `#! index` | Auto-generated TOC |
| `#! spacer <mm>` | Vertical spacing |
| `#! pagebreak` | Page break |
| `#! hr` | Horizontal rule |

**Template variables:** `findings`, `date`, `asset`, `from`, `to`, `total`, `critical`, `high`, `medium`, `low`, `info`.

See [`examples/report_template_example/template.tmpl`](examples/report_template_example/template.tmpl) for a complete working template.

### `pog update-status`

Update the status of a finding by its ID (folder name).

```
pog update-status -i <id> -S <status>
```

Valid statuses: `Open`, `InProgress`, `Resolved`, `FalsePositive`.

```
$ pog update-status -i sql-injection -S Resolved
[+] SQL Injection (nexus_portal) → Resolved

$ pog update-status -i weak-tls -S FalsePositive
[+] Weak TLS Configuration (orion_gateway) → False Positive
```

### `pog clean`

Wipe the database and all stored findings/assets.

```
$ pog clean
[+] Database and findings directory wiped clean
```

---

## Finding format

Each finding is a folder with one `.md` file and an optional `img/` directory:

```
sql-injection/
├── finding.md
└── img/
    └── proof.png
```

**Markdown template:**

```markdown
# SQL Injection

- **Severity:** Critical
- **Asset:** nexus_portal
- **Location:** https://portal.nexus.corp/api/users?id=1
- **Status:** Open
- **Date:** 2026/01/15

## Description

User input is directly concatenated into the SQL query without sanitisation.
```

| Field | Required | Default | Values |
|-------|----------|---------|--------|
| `# Title` | yes | folder name | — |
| `Severity` | no | `Info` | `Critical`, `High`, `Medium`, `Low`, `Info` |
| `Asset` | no | `unknown` | lowercase, underscores for spaces |
| `Location` | no | *(empty)* | URL, path, etc. |
| `Status` | no | `Open` | `Open`, `InProgress`, `Resolved`, `FalsePositive` |
| `Date` | no | *(empty)* | `YYYY/MM/DD` |
| `Description` | no | *(empty)* | Everything under `## Description` |

---

## Asset format

Assets are defined in Markdown files. Only the **name** is required — all other fields default to `-`.

**Single asset** (`asset.md`):

```markdown
# nexus_portal

- **Description:** Customer-facing web portal for Nexus Corp
- **Contact:** Platform Team <platform@nexus.corp>
- **Criticality:** Critical
- **DNS/IP:** portal.nexus.corp
```

**Bulk import** — multiple assets in one file separated by `---` (`assets.md`):

```markdown
# nexus_portal

- **Description:** Customer-facing web portal for Nexus Corp
- **Contact:** Platform Team <platform@nexus.corp>
- **Criticality:** Critical
- **DNS/IP:** portal.nexus.corp

---

# orion_gateway

- **Description:** API gateway for Orion services
- **Contact:** Infrastructure Team <infra@orion.corp>
- **Criticality:** Critical
- **DNS/IP:** gw.orion.corp

---

# helix_mobile

- **Description:** Mobile backend API for Helix app
- **Contact:** Mobile Team <mobile@helix.corp>
- **Criticality:** High
- **DNS/IP:** mobile-api.helix.corp
```

| Field | Required | Default | Notes |
|-------|----------|---------|-------|
| `# Name` | **yes** | — | Normalised to lowercase with underscores |
| `Description` | no | `-` | Free-text description |
| `Contact` | no | `-` | Responsible team / person |
| `Criticality` | no | `-` | e.g. `Critical`, `High`, `Medium`, `Low` |
| `DNS/IP` | no | `-` | Hostname or IP address |

---

## POGDIR – internal file structure

All data lives under **POGDIR** (`$POGDIR` env var or `$HOME/.pog`):

```
~/.pog/
├── pog.db                              # SQLite database (findings + assets)
└── findings/
    ├── nexus_portal/
    │   ├── asset.md                     # asset metadata
    │   ├── 0x001_sql-injection/
    │   │   ├── finding.md
    │   │   └── img/
    │   │       └── proof.png
    │   └── 0x002_open-redirect/
    │       └── finding.md
    └── orion_gateway/
        ├── asset.md
        └── 0x001_weak-tls/
            └── finding.md
```

Asset metadata is stored both in the SQLite database and as an `asset.md` file in each asset directory. You can browse with standard Unix tools:

```bash
ls ~/.pog/findings/                        # list assets
tree ~/.pog/findings/                      # full tree
grep -rl "SQL" ~/.pog/findings/            # keyword search
```

---

## Building

Builds run inside a **Podman container** for reproducibility.

```bash
make pod-build    # build container (first time)
make release      # compile & strip
make test         # run tests
make debug        # debug build
make clean        # clean artifacts
```

Requires [Podman](https://podman.io). No local Rust toolchain needed.

### Project structure

```
pog/
├── src/          # binary entry point
├── cli/          # CLI parsing (clap)
├── models/       # domain types – Finding, Asset, Severity, GraphData
├── storage/      # POGDIR layout, SQLite, import & report logic
├── tui/          # ratatui TUI (tabs: Graph, Search, Assets)
├── examples/     # sample findings & report template
├── Dockerfile    # Podman build container
├── Makefile      # build targets
└── Cargo.toml    # workspace root
```

---

## License

MIT – see [LICENSE](LICENSE).
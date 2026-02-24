# pog

A pentester's log – track, organise and browse security findings and assets from the terminal.

Findings and assets are plain **Markdown files**. Use `pog` to import, search and visualise them, **or** browse the file tree with standard Unix tools.

---

## Demo
[Screencast_20260210_164739.webm](https://github.com/user-attachments/assets/c7552340-7074-4a78-ba5d-b80dee208e34)

---

## Features

| Feature | Description |
|---------|-------------|
| **Markdown-native** | Plain `.md` files – read, edit and version-control with any tool. |
| **Asset management** | Track assets with metadata: name, description, contact, criticality, DNS/IP. |
| **Asset-based organisation** | Findings grouped by target asset with unique hex IDs. |
| **Interactive TUI** | Tabbed dashboard (Graph, Search, Assets) with keyboard & mouse. |
| **PDF reports** | Self-contained template-driven PDF reports via [Typst](https://typst.app/). Templates own their styling, images and layout; the program fills in findings and metadata. |
| **CSV export** | One-command export of all findings. |
| **Bulk import** | Batch-import findings or assets in one go. |
| **Multi-client** | Each client gets its own isolated DB and findings directory. Switch with `--client` or set a default. |
| **Upsert on re-import** | Re-importing a finding (same slug) updates the existing record. |
| **Sample data** | TUI shows demo findings & assets when the database is empty. |
| **Zero config** | Data lives in `~/.pog` (or `$POGDIR`). |

---

## Quick start

```bash
# Build
cargo build --release

# Client management
pog client create acme-corp
pog client create globex
pog client default acme-corp
pog client list

# Import findings (uses default client, or override with -c)
pog import-findings -p ./sql-injection
pog import-findings -p ./examples/findings_example --bulk
pog -c globex import-findings -p ./findings --bulk   # target a specific client

# Import assets
pog import-assets -p ./asset.md
pog import-assets -p ./assets.md --bulk

# TUI dashboard
pog view

# Export & report
pog export -o findings.csv -a nexus_portal --from 2025/09/01 --to 2026/01/31
pog report -t template.typ --asset nexus_portal --from 2025/09/01 --to 2026/01/31

# Wipe current client's data
pog clean
```

---

## Global flags

| Flag | Description |
|------|-------------|
| `-c`, `--client <name>` | Target a specific client. Overrides the default set by `pog client default`. |

All data-bearing commands (`import-findings`, `import-assets`, `view`, `export`, `report`, `update-status`, `clean`) respect this flag. If omitted, the current default client is used.

---

## Commands

### `pog client`

Manage clients. Each client gets its own isolated `pog.db` and `findings/` directory.

```
pog client create <name>      # create a new client
pog client list               # list all clients
pog client default <name>     # set the default client
pog client default            # show the current default
pog client delete <name>      # delete a client and all its data
```

```
$ pog client create acme-corp
[+] Created client: acme-corp

$ pog client create globex
[+] Created client: globex

$ pog client default acme-corp
[+] Default client set to: acme-corp

$ pog client list
[*] acme-corp (default)
[*] globex
```

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
| **Graph** | Severity distribution bars, Braille line chart (weekly timeline), severity toggle filters. |
| **Search** | Full-text search with severity / asset dropdown filters and detail panel. |
| **Assets** | Searchable asset list with criticality dropdown filter and detail panel. |

**Keyboard shortcuts:**

*Global:*

| Key | Action |
|-----|--------|
| `Tab` / `t` | Switch tab |
| `q` / `Esc` | Quit (when nothing is focused) |

*Graph tab:*

| Key | Action |
|-----|--------|
| `↑` / `k` | Move cursor up (severity filters) |
| `↓` / `j` | Move cursor down (severity filters) |
| `Space` / `Enter` | Toggle selected severity filter |

*Search tab:*

| Key | Action |
|-----|--------|
| `s` | Focus search box |
| `f` | Toggle severity filter dropdown |
| `a` | Toggle asset filter dropdown |
| `↑` / `↓` | Navigate finding list |
| `Esc` | Unfocus search / close dropdown |

*Assets tab:*

| Key | Action |
|-----|--------|
| `s` | Focus search box |
| `f` | Toggle criticality filter dropdown |
| `↑` / `k` | Move up |
| `↓` / `j` | Move down |
| `Esc` | Unfocus search / close dropdown |

### `pog export`

```
pog export                                                         # all findings → findings.csv
pog export -o report.csv                                           # custom output path
pog export -a nexus_portal                                         # only findings for an asset
pog export -a nexus_portal --from 2025/09/01 --to 2026/01/31       # asset + date range
pog export --from 2025/09/01 --to 2026/01/31                       # date range across all assets
```

| Flag | Description | Default |
|------|-------------|---------|
| `-o` | Output CSV path | `findings.csv` |
| `-a` | Filter by asset name | *(all)* |
| `--from` | Start date (`YYYY/MM/DD`) | *(unbounded)* |
| `--to` | End date (`YYYY/MM/DD`) | *(unbounded)* |

### `pog report`

Generate a PDF report from findings using a [Typst](https://typst.app/) template. No external installation is required — the Typst compiler is embedded in the binary.

Templates are **self-contained** – they own their styling, cover-page images and layout. Place assets (images, logos, etc.) alongside the template file. The entire template directory is copied into the build context so all relative paths resolve correctly.

```
pog report -t template.typ --asset nexus_portal --from 2025/09/01 --to 2026/01/31
```

| Flag | Description | Default |
|------|-------------|---------|
| `-t` | Template file (`.typ`) | *(required)* |
| `-o` | Output PDF path | `report.pdf` |
| `--asset` | Asset name | *(required)* |
| `--from` | Start date (`YYYY/MM/DD`) | *(required)* |
| `--to` | End date (`YYYY/MM/DD`) | *(required)* |

#### Template inputs

The following variables are injected into the template via `sys.inputs`:

| Variable | Type | Description |
|----------|------|-------------|
| `findings` | array of dicts | Each dict has: `num`, `title`, `severity`, `asset`, `date`, `location`, `report-content`, `status`, `images` |
| `date` | string | Report generation date (`YYYY/MM/DD`) |
| `asset` | string | Asset name |
| `from` / `to` | string | Assessment date range |
| `total` | int | Total finding count |
| `critical`, `high`, `medium`, `low`, `info` | int | Per-severity counts |

Finding `report-content` is automatically converted from Markdown to Typst markup. Use `eval(f.at("report-content"), mode: "markup")` in templates to render it.

**Example templates:**

- [`examples/report_template_example/template.typ`](examples/report_template_example/template.typ) – minimal working template

#### Template directory structure

Place images and other assets alongside the template file:

```
my_template/
├── template.typ         # the template
└── img/
    ├── banner.png       # referenced via image("img/banner.png")
    └── logo.png         # referenced via image("img/logo.png")
```

### `pog update-status`

Update the status of a finding by its asset and hex ID.

```
pog update-status -a <asset> -i <hex_id> -S <status>
```

Valid statuses: `Open`, `InProgress`, `Resolved`, `FalsePositive`.

```
$ pog update-status -a nexus_portal -i 0x001 -S Resolved
[+] SQL Injection [0x001] (nexus_portal) → Resolved

$ pog update-status -a orion_gateway -i 0x003 -S FalsePositive
[+] Weak TLS Configuration [0x003] (orion_gateway) → False Positive
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
---
title: SQL Injection
severity: Critical
asset: nexus_portal
location: https://portal.nexus.corp/api/users?id=1
status: Open
date: 2026/01/15
---

User input is directly concatenated into the SQL query without sanitisation.
```

| Field | Required | Default | Values |
|-------|----------|---------|--------|
| `title` | no | folder name | — |
| `severity` | no | `Info` | `Critical`, `High`, `Medium`, `Low`, `Info` |
| `asset` | no | `unknown` | lowercase, underscores for spaces |
| `location` | no | *(empty)* | URL, path, etc. |
| `status` | no | `Open` | `Open`, `InProgress`, `Resolved`, `FalsePositive` |
| `date` | no | *(empty)* | `YYYY/MM/DD` |
| Report content | no | *(empty)* | Everything after the closing `---` fence |

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

All data lives under **POGDIR** (`$POGDIR` env var or `$HOME/.pog`). Each client has its own isolated sub-directory:

```
~/.pog/
├── default_client                       # plain-text file with the active client name
└── clients/
    ├── acme-corp/
    │   ├── pog.db                       # SQLite database (findings + assets)
    │   └── findings/
    │       ├── nexus_portal/
    │       │   ├── asset.md
    │       │   ├── 0x001_sql-injection/
    │       │   │   ├── finding.md
    │       │   │   └── img/
    │       │   │       └── proof.png
    │       │   └── 0x002_open-redirect/
    │       │       └── finding.md
    │       └── orion_gateway/
    │           ├── asset.md
    │           └── 0x001_weak-tls/
    │               └── finding.md
    └── globex/
        ├── pog.db
        └── findings/
            └── ...
```

Asset metadata is stored both in the SQLite database and as an `asset.md` file in each asset directory. You can browse with standard Unix tools:

```bash
ls ~/.pog/clients/                                   # list clients
ls ~/.pog/clients/acme-corp/findings/                # list assets for a client
tree ~/.pog/clients/acme-corp/findings/              # full tree
grep -rl "SQL" ~/.pog/clients/acme-corp/findings/    # keyword search
```

---

## Building

Requires a Rust toolchain (edition 2024).

```bash
cargo build --release      # compile
cargo test --workspace     # run tests
```

The release binary is at `target/release/pog`.

### Project structure

```
pog/                          (Rust 2024 edition)
├── src/          # binary entry point
├── cli/          # CLI parsing (clap)
├── models/       # domain types – Finding, Asset, Severity, Status, GraphData
├── storage/      # POGDIR layout, SQLite (rusqlite), import, CSV export & PDF report (Typst)
├── tui/          # ratatui + crossterm TUI (tabs: Graph, Search, Assets)
├── examples/     # sample findings, assets & report template
└── Cargo.toml    # workspace root
```

---

## License

MIT – see [LICENSE](LICENSE).

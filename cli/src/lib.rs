use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pog")]
#[command(version)]
#[command(author = "Pengrey")]
pub struct Cli {
    /// Client / project name (overrides the default client)
    #[arg(short, long, global = true)]
    pub client: Option<String>,

    /// the command to execute
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Import finding(s) from a folder
    ImportFindings {
        /// Path to the finding folder (or parent folder when using --bulk)
        #[arg(short, long)]
        path: String,

        /// Treat <path> as a directory of finding folders and import them all
        #[arg(short, long, default_value_t = false)]
        bulk: bool,
    },

    /// Import asset(s) from a Markdown file
    ImportAssets {
        /// Path to the asset Markdown file
        #[arg(short, long)]
        path: String,

        /// Treat the file as containing multiple assets separated by ---
        #[arg(short, long, default_value_t = false)]
        bulk: bool,
    },

    /// View all findings and assets through a TUI
    View {},

    /// Generate a PDF report from findings
    Report {
        /// Report template file (.tmpl)
        #[arg(short, long)]
        template: String,

        /// Path to the output PDF file
        #[arg(short, long, default_value = "report.pdf")]
        output: String,

        /// Asset name to report on
        #[arg(short, long)]
        asset: String,

        /// Start date for the date range (YYYY/MM/DD)
        #[arg(long)]
        from: String,

        /// End date for the date range (YYYY/MM/DD)
        #[arg(long)]
        to: String,
    },

    /// Update the status of a finding
    UpdateStatus {
        /// Asset name, e.g. nexus_portal
        #[arg(short, long)]
        asset: String,

        /// Finding hex ID, e.g. 0x001
        #[arg(short, long)]
        id: String,

        /// New status: Open, InProgress, Resolved, FalsePositive
        #[arg(short = 'S', long)]
        status: String,
    },

    /// Wipe the database and all stored findings
    Clean {},

    /// Export all findings to CSV
    Export {
        /// Path to the output CSV file
        #[arg(short, long, default_value = "findings.csv")]
        output: String,

        /// Filter by asset name
        #[arg(short, long)]
        asset: Option<String>,

        /// Start date for the date range (YYYY/MM/DD)
        #[arg(long)]
        from: Option<String>,

        /// End date for the date range (YYYY/MM/DD)
        #[arg(long)]
        to: Option<String>,
    },

    /// Manage clients (each client gets its own DB and findings)
    Client {
        #[command(subcommand)]
        action: ClientAction,
    },
}

/// Sub-commands for `pog client …`
#[derive(Subcommand)]
pub enum ClientAction {
    /// Create a new client
    Create {
        /// Client name (used as directory name)
        name: String,
    },

    /// List all clients
    List,

    /// Delete a client and all its data
    Delete {
        /// Client name to delete
        name: String,
    },

    /// Set or show the default client
    Default {
        /// Client name to set as default (omit to show current default)
        name: Option<String>,
    },
}

pub fn parse_args() -> Cli {
    Cli::parse()
}
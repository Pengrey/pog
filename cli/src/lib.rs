use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pog")]
#[command(version)]
#[command(author = "Pengrey")]
pub struct Cli {
    /// the command to execute
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Import finding(s) from a folder
    Import {
        /// Path to the finding folder (or parent folder when using --bulk)
        #[arg(short, long)]
        path: String,

        /// Treat <path> as a directory of finding folders and import them all
        #[arg(short, long, default_value_t = false)]
        bulk: bool,
    },

    /// View all findings through a TUI
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

    /// Wipe the database and all stored findings
    Clean {},

    /// Export all findings to CSV
    Export {
        /// Path to the output CSV file
        #[arg(short, long, default_value = "findings.csv")]
        output: String,
    },
}

pub fn parse_args() -> Cli {
    Cli::parse()
}
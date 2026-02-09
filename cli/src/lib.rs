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

    /// Generate a report from the findings
    Report {
        /// the template to use for the report
        #[arg(short, long, required = true)]
        template: String,

        /// the path to the output file
        #[arg(short, long)]
        output: String,
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
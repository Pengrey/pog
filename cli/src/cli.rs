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
    /// Import a finding from a folder
    Import {
        /// the path to the folder containing the finding
        #[arg(short, long)]
        path: String,
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
}

pub fn parse_args() -> Cli {
    Cli::parse()
}
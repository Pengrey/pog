use std::path::Path;
use std::process;

#[macro_use]
mod log;

use cli::{parse_args, Commands};
use models::{GraphData, Severity, SeverityBar};
use storage::PogDir;

fn main() {
    if let Err(e) = run() {
        error!("{e}");
        process::exit(1);
    }
}

fn run() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args = parse_args();

    // Initialise POGDIR (creates dirs + DB on first run).
    let pog = PogDir::init()?;

    match args.command {
        Commands::Import { path, bulk } => {
            let folder = Path::new(&path);
            if bulk {
                let findings = storage::import_bulk(folder, &pog)?;
                success!("Imported {} finding(s)", findings.len());
                for f in &findings {
                    info!("{} [{}] ({})", f.title, f.severity, f.asset);
                }
            } else {
                let finding = storage::import_finding(folder, &pog)?;
                success!("Imported: {} [{}] ({})", finding.title, finding.severity, finding.asset);
            }
        }

        Commands::View {} => {
            let db = pog.open_db()?;
            let findings = db.all_findings()?;

            let graph_data = if findings.is_empty() {
                GraphData::sample_severity()
            } else {
                build_graph_data(&findings)
            };

            let display_findings = if findings.is_empty() {
                models::Finding::sample_findings()
            } else {
                findings
            };

            tui::run_with_data(graph_data, display_findings)?;
        }

        Commands::Report { output, template, asset, from, to } => {
            let db = pog.open_db()?;
            let findings = db.findings_filtered(
                Some(asset.as_str()),
                Some(from.as_str()),
                Some(to.as_str()),
            )?;

            if findings.is_empty() {
                error!("No findings match the given filters");
                process::exit(1);
            }

            info!(
                "Generating report for {} finding(s)…",
                findings.len()
            );

            storage::generate_report(
                Path::new(&template),
                Path::new(&output),
                &findings,
                &asset,
                &from,
                &to,
            )?;

            success!("Report written to {}", output);
        }

        Commands::Clean {} => {
            pog.clean()?;
            success!("Database and findings directory wiped clean");
        }

        Commands::Export { output } => {
            let db = pog.open_db()?;
            let csv = db.export_csv()?;
            std::fs::write(&output, &csv)?;
            success!("Exported findings to {}", output);
        }
    }

    Ok(())
}

/// Build a `GraphData` from the severity distribution of real findings.
fn build_graph_data(findings: &[models::Finding]) -> GraphData {
    let mut data = GraphData::new("Severity Distribution");
    for &sev in Severity::ALL {
        let count = findings.iter().filter(|f| f.severity == sev).count() as u64;
        if count > 0 {
            data = data.with_bar(SeverityBar::from_severity(sev, count));
        }
    }
    data
}
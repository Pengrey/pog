use std::path::Path;
use std::process;

#[macro_use]
mod log;

use cli::{parse_args, ClientAction, Commands};
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

    // ---- Handle client-management commands (no PogDir needed) --------
    if let Commands::Client { action } = args.command {
        return handle_client_action(action);
    }

    // ---- Resolve the client and initialise its POGDIR ----------------
    let client_name = PogDir::resolve_client(args.client.as_deref())?;
    let pog = PogDir::init_for_client(&client_name)?;

    match args.command {
        // (Client was already handled above.)
        Commands::Client { .. } => unreachable!(),

        Commands::ImportFindings { path, bulk } => {
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

        Commands::ImportAssets { path, bulk } => {
            let file = Path::new(&path);
            if bulk {
                let assets = storage::import_assets_bulk(file, &pog)?;
                success!("Imported {} asset(s)", assets.len());
                for a in &assets {
                    info!("{} [{}] ({})", a.name, a.criticality, a.dns_or_ip);
                }
            } else {
                let asset = storage::import_asset(file, &pog)?;
                success!("Imported asset: {} [{}]", asset.name, asset.criticality);
            }
        }

        Commands::View {} => {
            let db = pog.open_db()?;
            let findings = db.all_findings()?;
            let assets = db.all_assets()?;
            let graph_data = build_graph_data(&findings);

            tui::run_with_data(graph_data, findings, assets)?;
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
                &findings,
                &template,
                &output,
                &asset,
                &from,
                &to,
                &pog,
            )?;

            success!("Report written to {}", output);
        }

        Commands::UpdateStatus { asset, id, status } => {
            let parsed: models::Status = status.parse()
                .map_err(|e: String| e)?;
            let db = pog.open_db()?;
            let title = db.update_finding_status(&asset, &id, parsed.as_str())?;
            success!("{} [{}] ({}) → {}", title, id, asset, parsed);
        }

        Commands::Clean {} => {
            pog.clean()?;
            success!("Database and findings directory wiped clean");
        }

        Commands::Export { output, asset, from, to } => {
            let db = pog.open_db()?;
            let csv = db.export_csv(asset.as_deref(), from.as_deref(), to.as_deref())?;
            std::fs::write(&output, &csv)?;
            success!("Exported findings to {}", output);
        }
    }

    Ok(())
}

/// Handle `pog client <action>` sub-commands.
fn handle_client_action(action: ClientAction) -> std::result::Result<(), Box<dyn std::error::Error>> {
    match action {
        ClientAction::Create { name } => {
            PogDir::create_client(&name)?;
            success!("Created client: {}", name);
        }
        ClientAction::List => {
            let clients = PogDir::list_clients()?;
            let default = PogDir::get_default_client().ok();
            if clients.is_empty() {
                info!("No clients yet. Create one with `pog client create <name>`.");
            } else {
                for c in &clients {
                    if default.as_deref() == Some(c.as_str()) {
                        info!("{} (default)", c);
                    } else {
                        info!("{}", c);
                    }
                }
            }
        }
        ClientAction::Delete { name } => {
            PogDir::delete_client(&name)?;
            success!("Deleted client: {}", name);
        }
        ClientAction::Default { name } => {
            if let Some(name) = name {
                PogDir::set_default_client(&name)?;
                success!("Default client set to: {}", name);
            } else {
                match PogDir::get_default_client() {
                    Ok(current) => info!("Current default client: {}", current),
                    Err(_) => info!("No default client set. Use `pog client default <name>`."),
                }
            }
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
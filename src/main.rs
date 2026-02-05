use cli::{parse_args, Commands};
use tui::run;

fn main() -> std::io::Result<()> {
    let args = parse_args();

    match args.command {
        Commands::Import { path } => {
            println!("Importing finding from folder: {}", path);
        }

        Commands::View {} => {
            println!("Viewing findings through TUI");
            run()?;
        }

        Commands::Report { output, template } => {
            println!("Generating report with template: {} and output: {}", template, output);
        }
    }

    Ok(())
}
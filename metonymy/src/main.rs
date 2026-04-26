use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Optional command line argument
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.verbose {
        println!("Metonymy is running in verbose mode...");
    } else {
        println!("Metonymy is running...");
    }

    // Wire in the submodules eventually
    soundchange::parse_soundchange();

    Ok(())
}

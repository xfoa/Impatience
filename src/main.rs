mod cli;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Synctest(_args) => {
            // TODO: implement synctest logic
        }
    }
}

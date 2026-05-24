mod cli;

use clap::Parser;
use cli::interface::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Synctest(args) => {
            if let Some(host) = args.client {
                cli::synctest::client::run(
                    &host,
                    args.port,
                    args.count,
                    args.interval,
                    args.sync_interval,
                );
            } else if let Some(bind) = args.server {
                cli::synctest::server::run(&bind, args.port, args.sync_interval);
            } else {
                eprintln!("Error: must specify --server or --client");
                std::process::exit(1);
            }
        }
        Commands::Latencydemo(args) => {
            if let Some(host) = args.client {
                cli::latencydemo::client::run(&host, args.port, args.sync_interval, args.max_delay);
            } else if let Some(bind) = args.server {
                cli::latencydemo::server::run(&bind, args.port, args.sync_interval);
            } else {
                eprintln!("Error: must specify --server or --client");
                std::process::exit(1);
            }
        }
    }
}

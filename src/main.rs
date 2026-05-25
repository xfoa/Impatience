mod cli;

use clap::Parser;
use cli::interface::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::SyncTest(args) => {
            if let Some(host) = args.client {
                cli::sync_test::client::run(
                    &host,
                    args.port,
                    args.count,
                    args.interval,
                    args.sync_interval,
                );
            } else if let Some(bind) = args.server {
                cli::sync_test::server::run(&bind, args.port, args.sync_interval);
            } else {
                eprintln!("Error: must specify --server or --client");
                std::process::exit(1);
            }
        }
        Commands::LatencyDemo(args) => {
            if let Some(host) = args.client {
                cli::latency_demo::client::run(&host, args.port, args.sync_interval, args.max_delay);
            } else if let Some(bind) = args.server {
                cli::latency_demo::server::run(&bind, args.port, args.sync_interval);
            } else {
                eprintln!("Error: must specify --server or --client");
                std::process::exit(1);
            }
        }
    }
}

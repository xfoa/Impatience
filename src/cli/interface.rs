use clap::{ArgGroup, Parser, Subcommand};
use std::str::FromStr;

#[derive(Parser)]
#[command(name = "impatience", about = "A library for instrumentation of event-to-event latency over a network")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Synctest(Synctest),
}

#[derive(Parser)]
#[command(about = "Run a clock-synchronisation test over UDP")]
#[command(group = ArgGroup::new("mode").required(true).args(["server", "client"]))]
pub struct Synctest {
    #[arg(
        short = 's',
        long = "server",
        value_name = "BIND_ADDR",
        num_args = 0..=1,
        default_missing_value = "0.0.0.0",
        help = "Start a server on the specified bind address [0.0.0.0]"
    )]
    pub server: Option<String>,

    #[arg(
        short = 'c',
        long = "client",
        value_name = "HOST",
        help = "Connect to a server at the specified host"
    )]
    pub client: Option<String>,

    #[arg(
        short = 'p',
        long = "port",
        value_name = "PORT",
        default_value = "7340",
        help = "UDP port number"
    )]
    pub port: u16,

    #[arg(
        short = 'n',
        long = "count",
        value_name = "COUNT",
        default_value = "10",
        help = "Number of pings to send, or 'infinite'"
    )]
    pub count: Count,

    #[arg(
        short = 'i',
        long = "interval",
        value_name = "MS",
        default_value = "400",
        help = "Milliseconds between pings"
    )]
    pub interval: u64,

    #[arg(
        short = 'S',
        long = "sync-interval",
        value_name = "MS",
        default_value = "2000",
        help = "Milliseconds between sync heartbeats"
    )]
    pub sync_interval: u64,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum Count {
    Infinite,
    Finite(u64),
}

impl FromStr for Count {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("infinite") {
            Ok(Count::Infinite)
        } else {
            s.parse::<u64>()
                .map(Count::Finite)
                .map_err(|e| format!("invalid count '{}': {}", s, e))
        }
    }
}

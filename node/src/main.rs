mod config;
mod dashboard;
mod discovery;
mod error;
mod metrics;
mod node;
mod signaling;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "agora-node")]
#[command(about = "Agora Dedicated Node - P2P Voice Chat Infrastructure", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Start the node")]
    Start {
        #[arg(short, long, default_value = "/etc/agora/node.toml")]
        config: PathBuf,
        
        #[arg(short, long)]
        foreground: bool,
    },
    
    #[command(about = "Stop a running node")]
    Stop {
        #[arg(short, long, default_value = "/var/run/agora-node.pid")]
        pid_file: PathBuf,
    },
    
    #[command(about = "Show node status")]
    Status {
        #[arg(short, long, default_value = "http://localhost:8080")]
        endpoint: String,
    },
    
    #[command(about = "Generate default configuration")]
    Config {
        #[arg(short, long, default_value = "node.toml")]
        output: PathBuf,
    },
    
    #[command(about = "Generate a new identity")]
    Identity {
        #[arg(short, long, default_value = "identity.bin")]
        output: PathBuf,
        
        #[arg(short = 'n', long)]
        name: Option<String>,
    },
    
    #[command(about = "Discover available nodes")]
    Discover {
        #[arg(short, long, default_value = "http://localhost:8080")]
        endpoint: String,
        
        #[arg(short, long)]
        region: Option<String>,
        
        #[arg(short = 'c', long, default_value = "mixer")]
        capability: String,
    },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Start { config, foreground } => {
            node::run(config, foreground).await
        }
        Commands::Stop { pid_file } => {
            node::stop(pid_file).await
        }
        Commands::Status { endpoint } => {
            node::status(endpoint).await
        }
        Commands::Config { output } => {
            config::generate_default(&output)
        }
        Commands::Identity { output, name } => {
            node::generate_identity(&output, name).await
        }
        Commands::Discover { endpoint, region, capability } => {
            node::discover(endpoint, region, capability).await
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

use crate::config::NodeConfig;
use crate::dashboard::{Dashboard, DashboardData};
use crate::discovery::{NodeAdvertisement, NodeDiscovery, NodeMode};
use crate::error::NodeError;
use crate::metrics::NodeMetrics;
use agora_core::{Identity, IdentityStorage, NetworkNode};
use libp2p::PeerId;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;

pub async fn run(config_path: impl AsRef<Path>, _foreground: bool) -> Result<(), NodeError> {
    tracing::info!("Starting Agora Node...");

    let config = NodeConfig::load(config_path)?;
    tracing::info!("Loaded configuration: mode={}", config.node.mode);

    let identity =
        load_or_create_identity(&config.identity.key_file, config.identity.name.as_deref())?;
    tracing::info!("Identity loaded: {}", identity.peer_id());

    let metrics = Arc::new(NodeMetrics::new());
    let dashboard = Dashboard::new(&config);
    let dashboard_state = dashboard.state();

    let shutdown_token = tokio_util::sync::CancellationToken::new();
    let shutdown_token_clone = shutdown_token.clone();

    let network_handle = tokio::spawn(async move {
        run_network(
            config,
            identity,
            metrics,
            dashboard_state,
            shutdown_token_clone,
        )
        .await
    });

    let dashboard_handle = tokio::spawn(async move { dashboard.start().await });

    tracing::info!("Node started. Press Ctrl+C to stop.");

    match signal::ctrl_c().await {
        Ok(()) => {
            tracing::info!("Shutdown signal received...");
            shutdown_token.cancel();
        }
        Err(err) => {
            tracing::error!("Error waiting for shutdown signal: {}", err);
        }
    }

    network_handle.abort();
    dashboard_handle.abort();

    let _ = network_handle.await;
    let _ = dashboard_handle.await;

    tracing::info!("Node stopped.");
    Ok(())
}

async fn run_network(
    config: NodeConfig,
    identity: Identity,
    metrics: Arc<NodeMetrics>,
    dashboard_state: crate::dashboard::DashboardState,
    shutdown_token: tokio_util::sync::CancellationToken,
) -> Result<(), NodeError> {
    tracing::info!("Initializing network node...");

    let listen_addr = Some(config.listen_socket().to_string());
    let _network = NetworkNode::new(listen_addr.as_deref())
        .await
        .map_err(|e| NodeError::Network(format!("Failed to create network node: {}", e)))?;

    let node_mode = match config.node.mode {
        crate::config::NodeMode::Dedicated => NodeMode::Dedicated,
        crate::config::NodeMode::Relay => NodeMode::Relay,
        crate::config::NodeMode::Bootstrap => NodeMode::Bootstrap,
    };

    let peer_id = identity
        .peer_id()
        .parse::<PeerId>()
        .map_err(|e| NodeError::Identity(format!("Invalid peer ID: {}", e)))?;

    let mut advertisement = NodeAdvertisement::new(peer_id, node_mode);
    if let Some(ref region) = config.network.region {
        advertisement.region = region.clone();
    }

    let _discovery = Arc::new(tokio::sync::RwLock::new(NodeDiscovery::new()));

    {
        let mut data = dashboard_state.write().await;
        data.peer_id = identity.peer_id();
    }

    let mut interval = tokio::time::interval(Duration::from_secs(5));
    let mut advertise_interval = tokio::time::interval(Duration::from_secs(300));
    let mut uptime_secs: u64 = 0;

    loop {
        tokio::select! {
            _ = shutdown_token.cancelled() => {
                tracing::info!("Network shutting down...");
                break;
            }

            _ = interval.tick() => {
                uptime_secs += 5;
                metrics.update_uptime();

                advertisement.update_uptime(uptime_secs);

                let mut data = dashboard_state.write().await;
                data.status.uptime_seconds = uptime_secs;
                data.status.connections.total = 0;
                data.status.rooms.active = 0;
            }

            _ = advertise_interval.tick() => {
                tracing::debug!("Advertising node in DHT: {}", advertisement.dht_key());

                let _serialized = match advertisement.serialize() {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!("Failed to serialize advertisement: {}", e);
                        continue;
                    }
                };

                tracing::info!(
                    "Node advertisement: region={}, mode={}, load={}/{}",
                    advertisement.region,
                    advertisement.node_mode,
                    advertisement.current_load,
                    advertisement.max_clients
                );
            }
        }
    }

    Ok(())
}

fn load_or_create_identity(path: &str, name: Option<&str>) -> Result<Identity, NodeError> {
    let path = Path::new(path);

    if path.exists() {
        tracing::info!("Loading existing identity from {}", path.display());

        let parent = path.parent().unwrap_or(Path::new("."));
        let storage = IdentityStorage::with_path(parent.to_path_buf())
            .map_err(|e| NodeError::Identity(format!("Failed to create storage: {}", e)))?;

        storage
            .load()
            .map_err(|e| NodeError::Identity(format!("Failed to load identity: {}", e)))
    } else {
        tracing::info!("Creating new identity at {}", path.display());
        let mut identity = Identity::generate()
            .map_err(|e| NodeError::Identity(format!("Failed to generate identity: {}", e)))?;

        if let Some(name) = name {
            identity.set_display_name(name.to_string());
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| NodeError::Identity(format!("Failed to create directory: {}", e)))?;
        }

        let parent = path.parent().unwrap_or(Path::new("."));
        let storage = IdentityStorage::with_path(parent.to_path_buf())
            .map_err(|e| NodeError::Identity(format!("Failed to create storage: {}", e)))?;

        storage
            .save(&identity)
            .map_err(|e| NodeError::Identity(format!("Failed to save identity: {}", e)))?;

        Ok(identity)
    }
}

pub async fn stop(pid_file: impl AsRef<Path>) -> Result<(), NodeError> {
    let pid_path = pid_file.as_ref();

    if !pid_path.exists() {
        return Err(NodeError::Config("PID file not found".to_string()));
    }

    let pid: i32 = std::fs::read_to_string(pid_path)?
        .trim()
        .parse()
        .map_err(|e| NodeError::Config(format!("Invalid PID: {}", e)))?;

    tracing::info!("Stopping node with PID {}", pid);

    #[cfg(unix)]
    {
        use std::process::Command;
        Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .status()
            .map_err(|e| NodeError::Signal(format!("Failed to send signal: {}", e)))?;
    }

    println!("Stop signal sent to PID {}", pid);
    Ok(())
}

pub async fn status(endpoint: String) -> Result<(), NodeError> {
    let url = format!("{}/api/status", endpoint);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| NodeError::Network(format!("Failed to connect: {}", e)))?;

    if !response.status().is_success() {
        return Err(NodeError::Network(format!(
            "Status check failed: {}",
            response.status()
        )));
    }

    let status: DashboardData = response
        .json()
        .await
        .map_err(|e| NodeError::Network(format!("Failed to parse response: {}", e)))?;

    println!("Node Status:");
    println!("  Mode: {}", status.mode);
    println!("  Version: {}", status.version);
    println!("  Peer ID: {}", status.peer_id);
    println!("  Uptime: {}s", status.status.uptime_seconds);
    println!("  Connections: {}", status.status.connections.total);
    println!("  Rooms: {}", status.status.rooms.active);
    println!("  Participants: {}", status.status.rooms.participants);

    Ok(())
}

pub async fn generate_identity(
    output: impl AsRef<Path>,
    name: Option<String>,
) -> Result<(), NodeError> {
    let mut identity = Identity::generate()
        .map_err(|e| NodeError::Identity(format!("Failed to generate identity: {}", e)))?;

    if let Some(name) = name {
        identity.set_display_name(name);
    }

    let output = output.as_ref();
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let parent = output.parent().unwrap_or(Path::new("."));
    let storage = IdentityStorage::with_path(parent.to_path_buf())
        .map_err(|e| NodeError::Identity(format!("Failed to create storage: {}", e)))?;

    storage
        .save(&identity)
        .map_err(|e| NodeError::Identity(format!("Failed to save identity: {}", e)))?;

    println!("Generated new identity:");
    println!("  Peer ID: {}", identity.peer_id());
    println!("  Saved to: {}", output.display());

    Ok(())
}

pub async fn discover(
    endpoint: String,
    region: Option<String>,
    capability: String,
) -> Result<(), NodeError> {
    let url = format!("{}/api/discover", endpoint);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .query(&[
            ("region", region.as_deref()),
            ("capability", Some(capability.as_str())),
        ])
        .send()
        .await
        .map_err(|e| NodeError::Network(format!("Failed to connect: {}", e)))?;

    if !response.status().is_success() {
        return Err(NodeError::Network(format!(
            "Discovery failed: {}",
            response.status()
        )));
    }

    let nodes: Vec<NodeAdvertisement> = response
        .json()
        .await
        .map_err(|e| NodeError::Network(format!("Failed to parse response: {}", e)))?;

    if nodes.is_empty() {
        println!("No nodes discovered.");
        return Ok(());
    }

    println!("Discovered {} node(s):\n", nodes.len());

    for (i, node) in nodes.iter().enumerate() {
        println!("  [{}] Peer ID: {}", i + 1, node.peer_id);
        println!("      Region: {}", node.region);
        println!("      Mode: {}", node.node_mode);
        println!("      Capabilities: {:?}", node.capabilities);
        println!(
            "      Load: {}/{} ({:.1}%)",
            node.current_load,
            node.max_clients,
            node.load_percentage()
        );
        println!("      Uptime: {}s", node.uptime_seconds);
        println!("      Reputation: {:.2}", node.reputation);
        println!("      Score: {:.3}", node.score());
        println!();
    }

    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
#[allow(dead_code)]
pub struct DiscoveryResponse {
    pub nodes: Vec<NodeAdvertisement>,
}

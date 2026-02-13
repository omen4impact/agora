use clap::{Parser, Subcommand};
use agora_core::{Identity, NetworkNode, Room, RoomConfig};

#[derive(Parser)]
#[command(name = "agora")]
#[command(about = "Agora - Decentralized P2P Voice Chat CLI", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new identity
    Identity {
        /// Display name for the identity
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Create a new voice room
    CreateRoom {
        /// Room name
        #[arg(short, long)]
        name: Option<String>,
        /// Password protection
        #[arg(short, long)]
        password: Option<String>,
    },
    /// Start a network node
    StartNode {
        /// Listen port (0 for random)
        #[arg(short, long, default_value = "0")]
        port: u16,
        /// Bootstrap peers (multiaddr format)
        #[arg(short, long)]
        bootstrap: Option<String>,
    },
    /// Parse a room link
    ParseLink {
        /// Room link to parse
        link: String,
    },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Identity { name } => {
            handle_identity(name).await;
        }
        Commands::CreateRoom { name, password } => {
            handle_create_room(name, password).await;
        }
        Commands::StartNode { port, bootstrap } => {
            handle_start_node(port, bootstrap).await;
        }
        Commands::ParseLink { link } => {
            handle_parse_link(&link);
        }
    }
}

async fn handle_identity(name: Option<String>) {
    println!("Generating new identity...\n");
    
    let mut identity = Identity::generate()
        .expect("Failed to generate identity");
    
    if let Some(n) = name {
        identity.set_display_name(n);
    }
    
    println!("Peer ID:    {}", identity.peer_id());
    println!("Public Key: {}", identity.public_key_base64());
    
    if let Some(display_name) = identity.display_name() {
        println!("Name:       {}", display_name);
    }
    
    println!("\nKey bytes (save securely):");
    println!("{}", hex::encode(identity.to_bytes()));
}

async fn handle_create_room(name: Option<String>, password: Option<String>) {
    println!("Creating new room...\n");
    
    let identity = Identity::generate()
        .expect("Failed to generate identity");
    
    let config = RoomConfig {
        name,
        password,
        max_participants: Some(20),
    };
    
    let room = Room::new(identity.peer_id(), config);
    
    println!("Room ID:    {}", room.id);
    println!("Share Link: {}", room.share_link());
    
    if let Some(name) = &room.name {
        println!("Name:       {}", name);
    }
    
    if room.has_password() {
        println!("Protected:  Yes");
    } else {
        println!("Protected:  No (public room)");
    }
}

async fn handle_start_node(port: u16, bootstrap: Option<String>) {
    println!("Starting network node on port {}...\n", port);
    
    let listen_addr_str = format!("/ip4/0.0.0.0/tcp/{}", port);
    let listen_addr = if port == 0 {
        None
    } else {
        Some(listen_addr_str.as_str())
    };
    
    let mut node = NetworkNode::new(listen_addr).await
        .expect("Failed to start network node");
    
    println!("Local Peer ID: {}", node.peer_id_string());
    
    if let Some(bootstrap_addr) = bootstrap {
        println!("Connecting to bootstrap: {}", bootstrap_addr);
        let addr = agora_core::network::parse_multiaddr(&bootstrap_addr)
            .expect("Invalid bootstrap address");
        if let Err(e) = node.dial(addr).await {
            println!("Failed to connect to bootstrap: {}", e);
        }
    }
    
    println!("\nListening for connections... (Ctrl+C to stop)");
    
    loop {
        if let Some(event) = node.next_event().await {
            match event {
                agora_core::network::NetworkEvent::Listening(addr) => {
                    println!("[LISTENING] {}", addr);
                }
                agora_core::network::NetworkEvent::PeerConnected { peer_id, addr } => {
                    println!("[CONNECTED] {} at {}", peer_id, addr);
                }
                agora_core::network::NetworkEvent::PeerDisconnected { peer_id, .. } => {
                    println!("[DISCONNECTED] {}", peer_id);
                }
                agora_core::network::NetworkEvent::PeerIdentified { peer_id, listen_addrs } => {
                    println!("[IDENTIFIED] {} with {} addresses", peer_id, listen_addrs.len());
                    for addr in listen_addrs {
                        println!("  - {}", addr);
                    }
                }
                agora_core::network::NetworkEvent::ProvidersFound { room_id, providers } => {
                    println!("[PROVIDERS] Room {} has {} providers", room_id, providers.len());
                }
                agora_core::network::NetworkEvent::PingResult { peer_id, result } => {
                    match result {
                        Ok(()) => println!("[PING] {} OK", peer_id),
                        Err(e) => println!("[PING] {} FAILED: {}", peer_id, e),
                    }
                }
            }
        }
    }
}

fn handle_parse_link(link: &str) {
    println!("Parsing room link...\n");
    
    match agora_core::room::parse_room_link(link) {
        Some((room_id, password)) => {
            println!("Room ID:  {}", room_id);
            if let Some(pwd) = password {
                println!("Password: {}", pwd);
            } else {
                println!("Password: None (public room)");
            }
        }
        None => {
            println!("Invalid room link format");
            println!("Expected: agora://room/<id> or agora://room/<id>?p=<password>");
        }
    }
}

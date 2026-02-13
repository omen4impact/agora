use clap::{Parser, Subcommand};
use agora_core::{Identity, NetworkNode, Room, RoomConfig, SessionKey, EncryptedChannel};

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
        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,
    },
    /// Parse a room link
    ParseLink {
        /// Room link to parse
        link: String,
    },
    /// Test encryption
    TestEncrypt {
        /// Message to encrypt
        #[arg(short, long, default_value = "Hello, Agora!")]
        message: String,
    },
    /// Detect NAT type
    DetectNat,
}

#[tokio::main]
async fn main() {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(filter))
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Identity { name } => {
            handle_identity(name).await;
        }
        Commands::CreateRoom { name, password } => {
            handle_create_room(name, password).await;
        }
        Commands::StartNode { port, bootstrap, verbose } => {
            handle_start_node(port, bootstrap, verbose).await;
        }
        Commands::ParseLink { link } => {
            handle_parse_link(&link);
        }
        Commands::TestEncrypt { message } => {
            handle_test_encrypt(&message);
        }
        Commands::DetectNat => {
            handle_detect_nat().await;
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

async fn handle_start_node(port: u16, bootstrap: Option<String>, verbose: bool) {
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
    println!("Features enabled: AutoNAT, DCUtR (hole punch), Kademlia DHT\n");
    
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
                    if verbose {
                        for addr in listen_addrs {
                            println!("  - {}", addr);
                        }
                    }
                }
                agora_core::network::NetworkEvent::ProvidersFound { room_id, providers } => {
                    println!("[PROVIDERS] Room {} has {} providers", room_id, providers.len());
                }
                agora_core::network::NetworkEvent::PingResult { peer_id, result } => {
                    match result {
                        Ok(()) => {
                            if verbose {
                                println!("[PING] {} OK", peer_id);
                            }
                        }
                        Err(e) => println!("[PING] {} FAILED: {}", peer_id, e),
                    }
                }
                agora_core::network::NetworkEvent::NatStatusChanged { is_public } => {
                    if is_public {
                        println!("[NAT] Public IP detected - direct connections possible");
                    } else {
                        println!("[NAT] Behind NAT - hole punching enabled");
                    }
                }
                agora_core::network::NetworkEvent::BootstrapComplete => {
                    println!("[BOOTSTRAP] Kademlia bootstrap complete");
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

fn handle_test_encrypt(message: &str) {
    use agora_core::crypto::{generate_ephemeral_key, compute_fingerprint};
    
    println!("Testing E2E encryption...\n");
    
    // Generate ephemeral key
    let key_bytes = generate_ephemeral_key();
    let session_key = SessionKey::new(key_bytes);
    
    println!("Session Key: {}", hex::encode(key_bytes));
    println!("Fingerprint: {}", compute_fingerprint(&key_bytes));
    
    // Create encrypted channel
    let mut channel = EncryptedChannel::new(session_key);
    
    println!("\nEncrypting: \"{}\"", message);
    
    // Encrypt
    let encrypted = channel.encrypt(message.as_bytes())
        .expect("Encryption failed");
    
    println!("\nEncrypted Message:");
    println!("  Nonce:      {}", encrypted.nonce);
    println!("  Ciphertext: {}", hex::encode(&encrypted.ciphertext));
    println!("  Tag:        {}", hex::encode(encrypted.tag));
    
    // Decrypt
    let decrypted = channel.decrypt(&encrypted)
        .expect("Decryption failed");
    
    let decrypted_str = String::from_utf8(decrypted).expect("Invalid UTF-8");
    
    println!("\nDecrypted: \"{}\"", decrypted_str);
    
    // Test replay attack
    println!("\nTesting replay attack prevention...");
    match channel.decrypt(&encrypted) {
        Ok(_) => println!("  ERROR: Replay attack not detected!"),
        Err(e) => println!("  OK: Replay attack detected: {}", e),
    }
    
    // Test key expiry
    use std::time::Duration;
    let short_key = SessionKey::with_expiry([42u8; 32], Duration::from_millis(10));
    assert!(!short_key.is_expired());
    std::thread::sleep(Duration::from_millis(20));
    assert!(short_key.is_expired());
    println!("\nKey expiry test: PASSED");
}

async fn handle_detect_nat() {
    println!("Detecting NAT configuration...\n");
    
    use agora_core::NatTraversal;
    
    let mut nat = NatTraversal::new(None);
    
    println!("STUN servers configured:");
    for server in nat.get_stun_servers() {
        println!("  - {}", server);
    }
    
    match nat.detect_nat_type().await {
        Ok(nat_type) => {
            println!("\nNAT Type: {:?}", nat_type);
            println!("Description: {}", nat_type.description());
            println!("Can hole punch: {}", nat_type.can_hole_punch());
            
            if !nat_type.can_hole_punch() {
                println!("\n⚠ Your NAT type may require TURN relay for some connections.");
            } else {
                println!("\n✓ Direct P2P connections should work well.");
            }
        }
        Err(e) => {
            println!("\nNAT detection failed: {}", e);
            println!("This is expected without actual STUN connectivity.");
        }
    }
}

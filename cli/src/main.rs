use clap::{Parser, Subcommand};
use agora_core::{Identity, NetworkNode, Room, RoomConfig, SessionKey, EncryptedChannel, AudioPipeline, AudioConfig, AudioDevice};

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
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Create a new voice room
    CreateRoom {
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        password: Option<String>,
    },
    /// Start a network node
    StartNode {
        #[arg(short, long, default_value = "0")]
        port: u16,
        #[arg(short, long)]
        bootstrap: Option<String>,
        #[arg(short, long)]
        verbose: bool,
    },
    /// Parse a room link
    ParseLink {
        link: String,
    },
    /// Test encryption
    TestEncrypt {
        #[arg(short, long, default_value = "Hello, Agora!")]
        message: String,
    },
    /// Detect NAT type
    DetectNat,
    /// List audio devices
    ListAudioDevices,
    /// Test audio pipeline
    TestAudio {
        /// Duration in seconds
        #[arg(short, long, default_value = "5")]
        duration: u64,
        /// Enable noise suppression
        #[arg(short, long)]
        noise_suppression: bool,
    },
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
        Commands::ListAudioDevices => {
            handle_list_audio_devices();
        }
        Commands::TestAudio { duration, noise_suppression } => {
            handle_test_audio(duration, noise_suppression).await;
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
    
    let key_bytes = generate_ephemeral_key();
    let session_key = SessionKey::new(key_bytes);
    
    println!("Session Key: {}", hex::encode(key_bytes));
    println!("Fingerprint: {}", compute_fingerprint(&key_bytes));
    
    let mut channel = EncryptedChannel::new(session_key);
    
    println!("\nEncrypting: \"{}\"", message);
    
    let encrypted = channel.encrypt(message.as_bytes())
        .expect("Encryption failed");
    
    println!("\nEncrypted Message:");
    println!("  Nonce:      {}", encrypted.nonce);
    println!("  Ciphertext: {}", hex::encode(&encrypted.ciphertext));
    println!("  Tag:        {}", hex::encode(encrypted.tag));
    
    let decrypted = channel.decrypt(&encrypted)
        .expect("Decryption failed");
    
    let decrypted_str = String::from_utf8(decrypted).expect("Invalid UTF-8");
    
    println!("\nDecrypted: \"{}\"", decrypted_str);
    
    println!("\nTesting replay attack prevention...");
    match channel.decrypt(&encrypted) {
        Ok(_) => println!("  ERROR: Replay attack not detected!"),
        Err(e) => println!("  OK: Replay attack detected: {}", e),
    }
    
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

fn handle_list_audio_devices() {
    println!("Listing audio devices...\n");
    
    println!("📱 Input Devices:");
    match AudioDevice::input_devices() {
        Ok(devices) => {
            if devices.is_empty() {
                println!("  No input devices found");
            } else {
                for device in devices {
                    let default_marker = if device.is_default { " (default)" } else { "" };
                    println!("  • {}{}", device.name, default_marker);
                    println!("    Channels: {}, Sample Rate: {} Hz", device.channels, device.sample_rate);
                }
            }
        }
        Err(e) => println!("  Error: {}", e),
    }
    
    println!("\n🔊 Output Devices:");
    match AudioDevice::output_devices() {
        Ok(devices) => {
            if devices.is_empty() {
                println!("  No output devices found");
            } else {
                for device in devices {
                    let default_marker = if device.is_default { " (default)" } else { "" };
                    println!("  • {}{}", device.name, default_marker);
                    println!("    Channels: {}, Sample Rate: {} Hz", device.channels, device.sample_rate);
                }
            }
        }
        Err(e) => println!("  Error: {}", e),
    }
}

async fn handle_test_audio(duration: u64, noise_suppression: bool) {
    println!("Testing audio pipeline for {} seconds...\n", duration);
    
    let config = AudioConfig {
        enable_noise_suppression: noise_suppression,
        ..AudioConfig::default()
    };
    
    println!("Configuration:");
    println!("  Sample rate: {} Hz", config.sample_rate);
    println!("  Channels: {}", config.channels);
    println!("  Frame size: {} samples ({} ms)", config.frame_size, config.frame_size as f32 / config.sample_rate as f32 * 1000.0);
    println!("  Noise suppression: {}", if noise_suppression { "enabled" } else { "disabled" });
    println!();
    
    let mut pipeline = AudioPipeline::new(config);
    
    println!("Starting audio capture...");
    if let Err(e) = pipeline.start() {
        println!("Failed to start audio pipeline: {}", e);
        return;
    }
    
    println!("🎤 Speak into your microphone...\n");
    
    let start = std::time::Instant::now();
    let mut frame_count = 0u64;
    let mut total_rms = 0.0f32;
    
    while start.elapsed().as_secs() < duration {
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        
        if let Some(frame) = pipeline.capture_frame() {
            frame_count += 1;
            let rms = agora_core::audio::calculate_rms(&frame);
            total_rms += rms;
            
            let db = agora_core::audio::calculate_db(rms);
            let level = if db > -20.0 { "🔴" } else if db > -40.0 { "🟡" } else { "🟢" };
            
            print!("\r[Frame {:4}] RMS: {:.4} | dB: {:6.1} dB | {} ", frame_count, rms, db, level);
            use std::io::Write;
            std::io::stdout().flush().ok();
        }
    }
    
    println!("\n");
    pipeline.stop();
    
    let stats = pipeline.get_stats();
    println!("Audio Statistics:");
    println!("  Frames processed: {}", stats.frames_processed);
    println!("  Frames dropped: {}", stats.frames_dropped);
    
    if frame_count > 0 {
        let avg_rms = total_rms / frame_count as f32;
        let avg_db = agora_core::audio::calculate_db(avg_rms);
        println!("  Average level: {:.1} dB", avg_db);
    }
    
    println!("\n✓ Audio test complete");
}

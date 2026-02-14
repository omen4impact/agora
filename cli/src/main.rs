use agora_core::{
    AudioConfig, AudioDevice, AudioPipeline, EncryptedChannel, Identity, IdentityStorage,
    MixerConfig, MixerManager, NetworkNode, Room, RoomConfig, SessionKey,
};
use clap::{Parser, Subcommand};

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
    Identity {
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        load: bool,
        #[arg(short, long)]
        show: bool,
    },
    SaveIdentity {
        #[arg(short, long)]
        name: Option<String>,
    },
    DeleteIdentity,
    ExportIdentity {
        path: String,
    },
    ImportIdentity {
        path: String,
    },
    CreateRoom {
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        password: Option<String>,
    },
    StartNode {
        #[arg(short, long, default_value = "0")]
        port: u16,
        #[arg(short, long)]
        bootstrap: Option<String>,
        #[arg(short, long)]
        verbose: bool,
        #[arg(short, long)]
        room: Option<String>,
    },
    ParseLink {
        link: String,
    },
    TestEncrypt {
        #[arg(short, long, default_value = "Hello, Agora!")]
        message: String,
    },
    DetectNat,
    ListAudioDevices,
    TestAudio {
        #[arg(short, long, default_value = "5")]
        duration: u64,
        #[arg(short, long)]
        noise_suppression: bool,
    },
    TestMixer {
        #[arg(short, long, default_value = "6")]
        participants: usize,
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
        Commands::Identity { name, load, show } => handle_identity(name, load, show).await,
        Commands::SaveIdentity { name } => handle_save_identity(name).await,
        Commands::DeleteIdentity => handle_delete_identity().await,
        Commands::ExportIdentity { path } => handle_export_identity(&path).await,
        Commands::ImportIdentity { path } => handle_import_identity(&path).await,
        Commands::CreateRoom { name, password } => handle_create_room(name, password).await,
        Commands::StartNode {
            port,
            bootstrap,
            verbose,
            room,
        } => handle_start_node(port, bootstrap, verbose, room).await,
        Commands::ParseLink { link } => handle_parse_link(&link),
        Commands::TestEncrypt { message } => handle_test_encrypt(&message),
        Commands::DetectNat => handle_detect_nat().await,
        Commands::ListAudioDevices => handle_list_audio_devices(),
        Commands::TestAudio {
            duration,
            noise_suppression,
        } => handle_test_audio(duration, noise_suppression).await,
        Commands::TestMixer { participants } => handle_test_mixer(participants).await,
    }
}

async fn handle_identity(name: Option<String>, load: bool, show: bool) {
    let storage = match IdentityStorage::new() {
        Ok(s) => s,
        Err(e) => {
            println!("Error initializing storage: {}", e);
            return;
        }
    };

    if show {
        if !storage.has_stored_identity() {
            println!("No stored identity found.");
            println!("Run 'agora identity' to create a new one.");
            return;
        }

        match storage.load() {
            Ok(identity) => {
                println!("Stored Identity:\n");
                println!("Peer ID:    {}", identity.peer_id());
                println!("Public Key: {}", identity.public_key_base64());
                if let Some(display_name) = identity.display_name() {
                    println!("Name:       {}", display_name);
                }
                println!("\nConfig directory: {}", storage.config_dir().display());
            }
            Err(e) => println!("Error loading identity: {}", e),
        }
        return;
    }

    if load {
        match storage.load_or_create() {
            Ok(mut identity) => {
                if let Some(n) = name {
                    identity.set_display_name(n);
                    if let Err(e) = storage.save(&identity) {
                        println!("Warning: Failed to save name: {}", e);
                    }
                }

                println!("Identity loaded:\n");
                println!("Peer ID:    {}", identity.peer_id());
                println!("Public Key: {}", identity.public_key_base64());
                if let Some(display_name) = identity.display_name() {
                    println!("Name:       {}", display_name);
                }
            }
            Err(e) => println!("Error: {}", e),
        }
        return;
    }

    println!("Generating new identity...\n");

    let mut identity = Identity::generate().expect("Failed to generate identity");

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

    match storage.save(&identity) {
        Ok(_) => println!(
            "\nIdentity saved to: {}/identity.bin",
            storage.config_dir().display()
        ),
        Err(e) => println!("\nWarning: Failed to save identity: {}", e),
    }
}

async fn handle_save_identity(name: Option<String>) {
    let storage = match IdentityStorage::new() {
        Ok(s) => s,
        Err(e) => {
            println!("Error initializing storage: {}", e);
            return;
        }
    };

    let mut identity = match storage.load_or_create() {
        Ok(id) => id,
        Err(e) => {
            println!("Error: {}", e);
            return;
        }
    };

    if let Some(n) = name {
        identity.set_display_name(n);
    }

    match storage.save(&identity) {
        Ok(_) => {
            println!("Identity saved successfully.");
            println!("Peer ID: {}", identity.peer_id());
            if let Some(n) = identity.display_name() {
                println!("Name: {}", n);
            }
        }
        Err(e) => println!("Error saving identity: {}", e),
    }
}

async fn handle_delete_identity() {
    let storage = match IdentityStorage::new() {
        Ok(s) => s,
        Err(e) => {
            println!("Error initializing storage: {}", e);
            return;
        }
    };

    if !storage.has_stored_identity() {
        println!("No stored identity found.");
        return;
    }

    match storage.delete() {
        Ok(_) => println!("Identity deleted successfully."),
        Err(e) => println!("Error deleting identity: {}", e),
    }
}

async fn handle_export_identity(path: &str) {
    let storage = match IdentityStorage::new() {
        Ok(s) => s,
        Err(e) => {
            println!("Error initializing storage: {}", e);
            return;
        }
    };

    if !storage.has_stored_identity() {
        println!("No stored identity found.");
        println!("Run 'agora identity' first to create one.");
        return;
    }

    let identity = match storage.load() {
        Ok(id) => id,
        Err(e) => {
            println!("Error loading identity: {}", e);
            return;
        }
    };

    let export_path = std::path::Path::new(path);
    match storage.export_to_file(&identity, export_path) {
        Ok(_) => {
            println!("Identity exported to: {}", path);
            println!("Peer ID: {}", identity.peer_id());
            println!("\nWARNING: Keep this file secure!");
        }
        Err(e) => println!("Error exporting identity: {}", e),
    }
}

async fn handle_import_identity(path: &str) {
    let storage = match IdentityStorage::new() {
        Ok(s) => s,
        Err(e) => {
            println!("Error initializing storage: {}", e);
            return;
        }
    };

    if storage.has_stored_identity() {
        println!("Warning: This will replace your existing identity!");
        print!("Continue? [y/N] ");
        use std::io::{self, BufRead, Write};
        io::stdout().flush().ok();

        let mut input = String::new();
        io::stdin().lock().read_line(&mut input).ok();

        if !input.trim().to_lowercase().starts_with('y') {
            println!("Import cancelled.");
            return;
        }
    }

    let import_path = std::path::Path::new(path);
    match storage.import_from_file(import_path) {
        Ok(identity) => {
            println!("Identity imported successfully!");
            println!("Peer ID: {}", identity.peer_id());
            if let Some(n) = identity.display_name() {
                println!("Name: {}", n);
            }
        }
        Err(e) => println!("Error importing identity: {}", e),
    }
}

async fn handle_create_room(name: Option<String>, password: Option<String>) {
    println!("Creating new room...\n");

    let identity = Identity::generate().expect("Failed to generate identity");

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
    println!(
        "Protected:  {}",
        if room.has_password() {
            "Yes"
        } else {
            "No (public room)"
        }
    );
}

async fn handle_start_node(
    port: u16,
    bootstrap: Option<String>,
    verbose: bool,
    room: Option<String>,
) {
    println!("Starting network node on port {}...\n", port);

    let listen_addr_str = format!("/ip4/0.0.0.0/tcp/{}", port);
    let listen_addr = if port == 0 {
        None
    } else {
        Some(listen_addr_str.as_str())
    };

    let mut node = NetworkNode::new(listen_addr)
        .await
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

    if let Some(room_id) = &room {
        println!("Joining room: {}", room_id);
        if let Err(e) = node.start_providing(room_id).await {
            println!("Failed to join room: {}", e);
        } else {
            println!("Started providing room in DHT");
            node.get_providers(room_id);
        }
    }

    println!("\nListening for connections... (Ctrl+C to stop)");
    println!("Features: AutoNAT, DCUtR, Kademlia DHT\n");

    let mut event_rx = node.subscribe_events();

    tokio::spawn(async move {
        node.run().await;
    });

    while let Ok(event) = event_rx.recv().await {
        match event {
            agora_core::network::NetworkEvent::Listening(addr) => println!("[LISTENING] {}", addr),
            agora_core::network::NetworkEvent::PeerConnected { peer_id, addr } => {
                println!("[CONNECTED] {} at {}", peer_id, addr)
            }
            agora_core::network::NetworkEvent::PeerDisconnected { peer_id } => {
                println!("[DISCONNECTED] {}", peer_id)
            }
            agora_core::network::NetworkEvent::PeerIdentified {
                peer_id,
                listen_addrs,
            } => {
                println!(
                    "[IDENTIFIED] {} ({} addresses)",
                    peer_id,
                    listen_addrs.len()
                );
                if verbose {
                    for addr in listen_addrs {
                        println!("  - {}", addr);
                    }
                }
            }
            agora_core::network::NetworkEvent::ProvidersFound { room_id, providers } => {
                println!(
                    "[PROVIDERS] Room {} has {} providers",
                    room_id,
                    providers.len()
                )
            }
            agora_core::network::NetworkEvent::NatStatusChanged { is_public } => {
                println!(
                    "[NAT] {}",
                    if is_public {
                        "Public IP"
                    } else {
                        "Behind NAT - hole punching enabled"
                    }
                )
            }
            agora_core::network::NetworkEvent::BootstrapComplete => {
                println!("[BOOTSTRAP] Complete")
            }
            _ => {}
        }
    }
}

fn handle_parse_link(link: &str) {
    println!("Parsing room link...\n");

    match agora_core::room::parse_room_link(link) {
        Some((room_id, password)) => {
            println!("Room ID:  {}", room_id);
            match password {
                Some(pwd) => println!("Password: {}", pwd),
                None => println!("Password: None (public room)"),
            }
        }
        None => {
            println!("Invalid room link format");
            println!("Expected: agora://room/<id> or agora://room/<id>?p=<password>");
        }
    }
}

fn handle_test_encrypt(message: &str) {
    use agora_core::crypto::{compute_fingerprint, generate_ephemeral_key};

    println!("Testing E2E encryption...\n");

    let key_bytes = generate_ephemeral_key();
    let session_key = SessionKey::new(key_bytes);

    println!("Session Key: {}", hex::encode(key_bytes));
    println!("Fingerprint: {}", compute_fingerprint(&key_bytes));

    let mut channel = EncryptedChannel::new(session_key);
    println!("\nEncrypting: \"{}\"", message);

    let encrypted = channel
        .encrypt(message.as_bytes())
        .expect("Encryption failed");

    println!("\nEncrypted Message:");
    println!("  Nonce:      {}", hex::encode(encrypted.nonce));
    println!("  Ciphertext: {}", hex::encode(&encrypted.ciphertext));

    let decrypted = channel.decrypt(&encrypted).expect("Decryption failed");
    println!(
        "\nDecrypted: \"{}\"",
        String::from_utf8(decrypted).expect("Invalid UTF-8")
    );

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
            println!(
                "\n{}",
                if !nat_type.can_hole_punch() {
                    "⚠ Your NAT type may require TURN relay for some connections."
                } else {
                    "✓ Direct P2P connections should work well."
                }
            );
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
                    let marker = if device.is_default { " (default)" } else { "" };
                    println!("  • {}{}", device.name, marker);
                    println!(
                        "    Channels: {}, Sample Rate: {} Hz",
                        device.channels, device.sample_rate
                    );
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
                    let marker = if device.is_default { " (default)" } else { "" };
                    println!("  • {}{}", device.name, marker);
                    println!(
                        "    Channels: {}, Sample Rate: {} Hz",
                        device.channels, device.sample_rate
                    );
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
    println!("  Frame size: {} samples", config.frame_size);
    println!(
        "  Noise suppression: {}",
        if noise_suppression {
            "enabled"
        } else {
            "disabled"
        }
    );
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
            let level = if db > -20.0 {
                "🔴"
            } else if db > -40.0 {
                "🟡"
            } else {
                "🟢"
            };

            print!(
                "\r[Frame {:4}] RMS: {:.4} | dB: {:6.1} dB | {} ",
                frame_count, rms, db, level
            );
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

async fn handle_test_mixer(participant_count: usize) {
    println!(
        "Testing Mixer Algorithm with {} participants...\n",
        participant_count
    );

    let config = MixerConfig::default();
    println!("Configuration:");
    println!("  Full-Mesh threshold: {}", config.full_mesh_threshold);
    println!(
        "  Rotation interval: {} min",
        config.rotation_interval.as_secs() / 60
    );
    println!(
        "  Score weights: BW={:.0}%, Stab={:.0}%, Res={:.0}%, Dur={:.0}%",
        config.score_weights.bandwidth * 100.0,
        config.score_weights.stability * 100.0,
        config.score_weights.resources * 100.0,
        config.score_weights.duration * 100.0,
    );
    println!();

    let mut manager = MixerManager::new("local_peer".to_string(), Some(config));

    println!("Adding {} participants...\n", participant_count);

    for i in 0..participant_count {
        let peer_id = format!("peer_{}", i);
        manager.add_participant(peer_id.clone());

        // Simulate different stats for each peer
        let bandwidth = 1_000_000 + (i as u64 * 1_000_000);
        let cpu = 20.0 + (i as f32 * 10.0) % 60.0;
        let memory = 30.0 + (i as f32 * 5.0) % 40.0;

        manager.update_participant_stats(
            &peer_id,
            agora_core::mixer::ParticipantStats {
                bandwidth_bps: bandwidth,
                latency_ms: 20 + (i as u32 * 10),
                latency_variance: (i as f32 * 0.1) % 50.0,
                cpu_usage_percent: cpu,
                memory_usage_percent: memory,
                session_duration: std::time::Duration::from_secs((i as u64 + 1) * 600),
                ..Default::default()
            },
        );
    }

    let status = manager.get_status();
    println!("Topology: {:?}", status.topology);
    println!("Participant count: {}", status.participant_count);

    // Show all participants and their scores
    println!("\nParticipant Scores:");
    println!("┌─────────────┬───────────┬──────────┬─────────┬──────────┬────────┐");
    println!("│ Peer ID     │ Bandwidth │ Latency  │ CPU %   │ Duration │ Score  │");
    println!("├─────────────┼───────────┼──────────┼─────────┼──────────┼────────┤");

    for (peer_id, participant) in manager.get_participants() {
        let bw = participant.stats.bandwidth_bps / 1_000_000;
        let lat = participant.stats.latency_ms;
        let cpu = participant.stats.cpu_usage_percent as u32;
        let dur = participant.stats.session_duration.as_secs() / 60;
        let score = participant.score;

        println!(
            "│ {:11} │ {:4} Mbps │ {:4} ms  │ {:3} %   │ {:4} min  │ {:.4} │",
            peer_id, bw, lat, cpu, dur, score
        );
    }
    println!("└─────────────┴───────────┴──────────┴─────────┴──────────┴────────┘");

    // Local peer
    println!("\nLocal peer (simulated):");
    manager.update_local_stats(8_000_000, 25.0, 35.0);
    println!("  Bandwidth: 8 Mbps, CPU: 25%, Memory: 35%");

    // Select mixer
    let mixer = manager.select_mixer();
    let status = manager.get_status();

    println!("\nSelected Mixer: {}", mixer.as_deref().unwrap_or("None"));
    println!("Local is mixer: {}", status.is_local_mixer);

    // Show connection targets
    let targets = manager.get_connection_targets();
    println!(
        "\nConnection targets ({:?} mode): {} targets",
        status.topology,
        targets.len()
    );

    if status.topology == agora_core::mixer::TopologyMode::SFU {
        println!("  Target: {}", targets.join(", "));
    } else {
        for target in &targets {
            println!("  - {}", target);
        }
    }

    // Test rotation
    println!("\n--- Testing Mixer Rotation ---");
    let mixer_before = manager.get_current_mixer().map(|s| s.to_string());
    println!("Current mixer: {:?}", mixer_before);

    // Manually rotate
    let new_mixer = manager.rotate_mixer();
    println!("After rotation: {:?}", new_mixer);

    if mixer_before.as_deref() != new_mixer.as_deref() {
        println!("✓ Mixer changed successfully");
    } else {
        println!("⚠ Mixer stayed the same (might be the best candidate)");
    }

    // Test tie resolution
    println!("\n--- Testing Tie Resolution ---");
    println!(
        "When scores are within {:.0}% difference:",
        agora_core::mixer::SCORE_TIE_THRESHOLD * 100.0
    );
    println!("Resolution: Select lexicographically smallest peer ID");
    println!("This ensures all clients select the same mixer deterministically");

    // Test topology switching
    println!("\n--- Testing Topology Switching ---");
    let mut test_manager = MixerManager::new("test".to_string(), None);

    for i in 0..5 {
        test_manager.add_participant(format!("p{}", i));
        println!(
            "Participants: {} -> Mode: {:?}",
            test_manager.get_participant_count(),
            test_manager.get_topology_mode()
        );
    }

    test_manager.add_participant("p6".to_string());
    println!(
        "Participants: {} -> Mode: {:?}",
        test_manager.get_participant_count(),
        test_manager.get_topology_mode()
    );

    println!("\n✓ Mixer test complete");
}

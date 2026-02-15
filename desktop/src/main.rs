#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use agora_core::{
    protocol::ControlMessage, AudioConfig, AudioPipeline, MixerConfig, MixerManager,
    NetworkCommand, NetworkEvent, NetworkNode,
};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;

struct AppState {
    identity: Arc<Mutex<Option<agora_core::Identity>>>,
    network_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    network_command: Arc<Mutex<Option<mpsc::Sender<NetworkCommand>>>>,
    audio: Arc<Mutex<Option<AudioPipeline>>>,
    mixer: Arc<Mutex<Option<MixerManager>>>,
    current_room: Arc<Mutex<Option<RoomState>>>,
    participants: Arc<Mutex<HashMap<String, ParticipantInfo>>>,
    settings: Arc<Mutex<AppSettings>>,
    listen_addrs: Arc<Mutex<Vec<String>>>,
    connected_peers: Arc<Mutex<Vec<String>>>,
}

use tokio::sync::mpsc;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct AppSettings {
    audio: AudioSettings,
    network: NetworkSettings,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct AudioSettings {
    input_device: Option<String>,
    output_device: Option<String>,
    noise_suppression: bool,
    input_volume: f32,
    output_volume: f32,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct NetworkSettings {
    listen_port: Option<u16>,
    bootstrap_nodes: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            audio: AudioSettings {
                input_device: None,
                output_device: None,
                noise_suppression: true,
                input_volume: 1.0,
                output_volume: 1.0,
            },
            network: NetworkSettings {
                listen_port: None,
                bootstrap_nodes: vec![],
            },
        }
    }
}

#[derive(Clone, serde::Serialize)]
struct RoomState {
    id: String,
    name: Option<String>,
    link: String,
}

#[derive(Clone, serde::Serialize)]
struct ParticipantInfo {
    peer_id: String,
    display_name: Option<String>,
    is_mixer: bool,
    is_muted: bool,
    latency_ms: u32,
}

#[derive(Clone, serde::Serialize)]
struct RoomInfo {
    id: String,
    name: Option<String>,
    link: String,
    has_password: bool,
}

#[derive(Clone, serde::Serialize)]
struct AudioDevices {
    input: Vec<AudioDeviceInfo>,
    output: Vec<AudioDeviceInfo>,
}

#[derive(Clone, serde::Serialize)]
struct AudioDeviceInfo {
    name: String,
    is_default: bool,
    channels: u16,
    sample_rate: u32,
}

#[derive(Clone, serde::Serialize)]
struct MixerStatus {
    topology: String,
    participant_count: usize,
    current_mixer: Option<String>,
    is_local_mixer: bool,
    uptime_secs: Option<u64>,
}

#[derive(Clone, serde::Serialize)]
struct NetworkInfo {
    peer_id: String,
    listen_addrs: Vec<String>,
    connected_peers: Vec<String>,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let state = AppState {
        identity: Arc::new(Mutex::new(None)),
        network_handle: Arc::new(Mutex::new(None)),
        network_command: Arc::new(Mutex::new(None)),
        audio: Arc::new(Mutex::new(None)),
        mixer: Arc::new(Mutex::new(None)),
        current_room: Arc::new(Mutex::new(None)),
        participants: Arc::new(Mutex::new(HashMap::new())),
        settings: Arc::new(Mutex::new(AppSettings::default())),
        listen_addrs: Arc::new(Mutex::new(Vec::new())),
        connected_peers: Arc::new(Mutex::new(Vec::new())),
    };

    let result = tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            init_identity,
            get_peer_id,
            get_display_name,
            set_display_name,
            create_room,
            join_room,
            start_network,
            stop_network,
            start_audio,
            stop_audio,
            get_audio_devices,
            init_mixer,
            add_participant,
            remove_participant,
            get_mixer_status,
            get_room_info,
            get_participants,
            set_muted,
            export_identity,
            import_identity,
            save_settings,
            load_settings,
            get_settings,
            get_network_info,
            connect_peer,
        ])
        .run(tauri::generate_context!());

    if let Err(e) = result {
        eprintln!("Fatal error running Agora: {}", e);
        std::process::exit(1);
    }
}

#[tauri::command]
async fn init_identity(state: tauri::State<'_, AppState>) -> Result<String, String> {
    tracing::info!("[INIT] Generating new identity...");
    let identity = agora_core::Identity::generate()
        .map_err(|e| {
            tracing::error!("[INIT] Failed to generate identity: {}", e);
            format!("Failed to generate identity: {}", e)
        })?;
    let peer_id = identity.peer_id();
    tracing::info!("[INIT] Identity generated: {}", peer_id);

    let mut id_lock = state.identity.lock().await;
    *id_lock = Some(identity);
    tracing::info!("[INIT] Identity stored in state");

    Ok(peer_id)
}

#[tauri::command]
async fn get_peer_id(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let id_lock = state.identity.lock().await;
    id_lock
        .as_ref()
        .map(|i| i.peer_id())
        .ok_or_else(|| "Identity not initialized".to_string())
}

#[tauri::command]
async fn get_display_name(state: tauri::State<'_, AppState>) -> Result<Option<String>, String> {
    let id_lock = state.identity.lock().await;
    id_lock
        .as_ref()
        .map(|i| i.display_name().map(|s| s.to_string()))
        .ok_or_else(|| "Identity not initialized".to_string())
}

#[tauri::command]
async fn set_display_name(state: tauri::State<'_, AppState>, name: String) -> Result<(), String> {
    let mut id_lock = state.identity.lock().await;
    id_lock
        .as_mut()
        .map(|i| i.set_display_name(name))
        .ok_or_else(|| "Identity not initialized".to_string())
}

#[tauri::command]
async fn create_room(
    state: tauri::State<'_, AppState>,
    name: Option<String>,
    password: Option<String>,
) -> Result<RoomInfo, String> {
    let id_lock = state.identity.lock().await;
    let peer_id = id_lock
        .as_ref()
        .map(|i| i.peer_id())
        .ok_or_else(|| "Identity not initialized".to_string())?;
    drop(id_lock);

    let config = agora_core::RoomConfig {
        name,
        password,
        max_participants: Some(20),
    };

    let room = agora_core::Room::new(peer_id, config);
    let info = RoomInfo {
        id: room.id.clone(),
        name: room.name.clone(),
        link: room.share_link(),
        has_password: room.has_password(),
    };

    let room_id = room.id.clone();
    let link = info.link.clone();
    let name_clone = room.name.clone();

    {
        let mut room_lock = state.current_room.lock().await;
        *room_lock = Some(RoomState {
            id: room_id.clone(),
            name: name_clone,
            link: link.clone(),
        });
    }

    {
        let cmd_lock = state.network_command.lock().await;
        if let Some(cmd_tx) = cmd_lock.as_ref() {
            cmd_tx
                .send(NetworkCommand::JoinRoom { room_id })
                .await
                .map_err(|e| format!("Failed to join room: network unavailable - {}", e))?;
        }
    }

    Ok(info)
}

#[tauri::command]
async fn join_room(state: tauri::State<'_, AppState>, room_link: String) -> Result<String, String> {
    let (room_id, _password) = agora_core::room::parse_room_link(&room_link)
        .ok_or_else(|| "Invalid room link".to_string())?;

    {
        let mut room_lock = state.current_room.lock().await;
        *room_lock = Some(RoomState {
            id: room_id.clone(),
            name: None,
            link: room_link,
        });
    }

    {
        let cmd_lock = state.network_command.lock().await;
        if let Some(cmd_tx) = cmd_lock.as_ref() {
            cmd_tx
                .send(NetworkCommand::JoinRoom {
                    room_id: room_id.clone(),
                })
                .await
                .map_err(|e| format!("Failed to join room: network unavailable - {}", e))?;
        }
    }

    Ok(room_id)
}

#[tauri::command]
async fn start_network(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    listen_port: Option<u16>,
) -> Result<String, String> {
    let listen_addr = listen_port
        .map(|p| format!("/ip4/0.0.0.0/tcp/{}", p))
        .unwrap_or_else(|| "/ip4/0.0.0.0/tcp/0".to_string());

    let mut network = NetworkNode::new(Some(&listen_addr))
        .await
        .map_err(|e| format!("Failed to start network: {}", e))?;

    let peer_id = network.peer_id_string();
    let listen_addrs: Vec<String> = network.listen_addrs().iter().map(|a| a.to_string()).collect();
    tracing::info!("[NETWORK] Started with peer_id: {}", peer_id);
    tracing::info!("[NETWORK] Listening on: {:?}", listen_addrs);

    let mut event_rx = network.subscribe_events();
    let cmd_tx = network.command_sender();

    let (_shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

    {
        let mut cmd_lock = state.network_command.lock().await;
        *cmd_lock = Some(cmd_tx.clone());
    }

    {
        let mut addrs_lock = state.listen_addrs.lock().await;
        *addrs_lock = listen_addrs;
    }

    let connected_peers = state.connected_peers.clone();
    let app_handle = app.clone();
    let handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                result = event_rx.recv() => {
                    match result {
                        Ok(event) => {
                            match event {
                                NetworkEvent::PeerConnected { peer_id, addr } => {
                                    let _ = app_handle.emit(
                                        "peer-connected",
                                        serde_json::json!({
                                            "peer_id": peer_id.to_string(),
                                            "addr": addr.to_string()
                                        }),
                                    );
                                    {
                                        let mut peers = connected_peers.lock().await;
                                        let peer_str = peer_id.to_string();
                                        if !peers.contains(&peer_str) {
                                            peers.push(peer_str);
                                        }
                                    }
                                }
                                NetworkEvent::PeerDisconnected { peer_id } => {
                                    let _ = app_handle.emit(
                                        "peer-disconnected",
                                        serde_json::json!({
                                            "peer_id": peer_id.to_string()
                                        }),
                                    );
                                    {
                                        let mut peers = connected_peers.lock().await;
                                        peers.retain(|p| p != &peer_id.to_string());
                                    }
                                }
                                NetworkEvent::ProvidersFound { room_id, providers } => {
                                    let _ = app_handle.emit("providers-found", serde_json::json!({
                                        "room_id": room_id,
                                        "providers": providers.iter().map(|p| p.to_string()).collect::<Vec<_>>()
                                    }));
                                }
                                NetworkEvent::AudioReceived { peer_id, packet } => {
                                    let _ = app_handle.emit(
                                        "audio-received",
                                        serde_json::json!({
                                            "peer_id": peer_id.to_string(),
                                            "sequence": packet.sequence
                                        }),
                                    );
                                }
                                NetworkEvent::RoomJoined { room_id, peer_id } => {
                                    let _ = app_handle.emit(
                                        "room-joined",
                                        serde_json::json!({
                                            "room_id": room_id,
                                            "peer_id": peer_id.to_string()
                                        }),
                                    );
                                }
                                NetworkEvent::RoomLeft { room_id, peer_id } => {
                                    let _ = app_handle.emit(
                                        "room-left",
                                        serde_json::json!({
                                            "room_id": room_id,
                                            "peer_id": peer_id.to_string()
                                        }),
                                    );
                                }
                                NetworkEvent::NatStatusChanged { is_public } => {
                                    let _ = app_handle.emit(
                                        "nat-status",
                                        serde_json::json!({
                                            "is_public": is_public
                                        }),
                                    );
                                }
                                NetworkEvent::Listening(addr) => {
                                    let _ = app_handle.emit(
                                        "listening",
                                        serde_json::json!({
                                            "addr": addr.to_string()
                                        }),
                                    );
                                }
                                NetworkEvent::Error(e) => {
                                    tracing::error!("Network error: {}", e);
                                    let _ = app_handle.emit(
                                        "network-error",
                                        serde_json::json!({ "error": e.to_string() }),
                                    );
                                }
                                _ => {}
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            tracing::info!("Event channel closed, exiting event loop");
                            break;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("Event channel lagged by {} messages", n);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    tracing::info!("Network event loop shutting down");
                    break;
                }
            }
        }
    });

    let _network_handle = tokio::spawn(async move {
        network.run().await;
    });

    {
        let mut handle_lock = state.network_handle.lock().await;
        *handle_lock = Some(handle);
    }

    Ok(peer_id)
}

#[tauri::command]
async fn stop_network(state: tauri::State<'_, AppState>) -> Result<(), String> {
    {
        let cmd_lock = state.network_command.lock().await;
        if let Some(cmd_tx) = cmd_lock.as_ref() {
            let _ = cmd_tx.send(NetworkCommand::Stop).await;
        }
    }

    {
        let mut handle_lock = state.network_handle.lock().await;
        if let Some(handle) = handle_lock.take() {
            handle.abort();
        }
    }

    {
        let mut room_lock = state.current_room.lock().await;
        *room_lock = None;
    }

    {
        let mut participants = state.participants.lock().await;
        participants.clear();
    }

    Ok(())
}

#[tauri::command]
async fn start_audio(
    state: tauri::State<'_, AppState>,
    noise_suppression: bool,
) -> Result<(), String> {
    let config = AudioConfig {
        enable_noise_suppression: noise_suppression,
        ..AudioConfig::default()
    };

    let mut audio = AudioPipeline::new(config);
    audio
        .start()
        .map_err(|e| format!("Failed to start audio: {}", e))?;

    if !audio.is_running() {
        return Err("Audio pipeline failed to start".to_string());
    }

    {
        let mut audio_lock = state.audio.lock().await;
        *audio_lock = Some(audio);
    }

    Ok(())
}

#[tauri::command]
async fn stop_audio(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut audio_lock = state.audio.lock().await;
    if let Some(audio) = audio_lock.as_mut() {
        audio.stop();
    }
    *audio_lock = None;
    Ok(())
}

#[tauri::command]
async fn get_audio_devices() -> Result<AudioDevices, String> {
    use agora_core::AudioDevice;

    let input =
        AudioDevice::input_devices().map_err(|e| format!("Failed to get input devices: {}", e))?;
    let output = AudioDevice::output_devices()
        .map_err(|e| format!("Failed to get output devices: {}", e))?;

    Ok(AudioDevices {
        input: input
            .into_iter()
            .map(|d| AudioDeviceInfo {
                name: d.name,
                is_default: d.is_default,
                channels: d.channels,
                sample_rate: d.sample_rate,
            })
            .collect(),
        output: output
            .into_iter()
            .map(|d| AudioDeviceInfo {
                name: d.name,
                is_default: d.is_default,
                channels: d.channels,
                sample_rate: d.sample_rate,
            })
            .collect(),
    })
}

#[tauri::command]
async fn init_mixer(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let peer_id = {
        let id_lock = state.identity.lock().await;
        id_lock
            .as_ref()
            .map(|i| i.peer_id())
            .ok_or_else(|| "Identity not initialized".to_string())?
    };

    let mixer = MixerManager::new(peer_id, Some(MixerConfig::default()));

    {
        let mut mixer_lock = state.mixer.lock().await;
        *mixer_lock = Some(mixer);
    }

    Ok(())
}

#[tauri::command]
async fn add_participant(state: tauri::State<'_, AppState>, peer_id: String) -> Result<(), String> {
    {
        let mut mixer_lock = state.mixer.lock().await;
        if let Some(mixer) = mixer_lock.as_mut() {
            mixer.add_participant(peer_id.clone());
        }
    }

    {
        let mut participants = state.participants.lock().await;
        participants.insert(
            peer_id.clone(),
            ParticipantInfo {
                peer_id,
                display_name: None,
                is_mixer: false,
                is_muted: false,
                latency_ms: 0,
            },
        );
    }

    Ok(())
}

#[tauri::command]
async fn remove_participant(
    state: tauri::State<'_, AppState>,
    peer_id: String,
) -> Result<(), String> {
    {
        let mut mixer_lock = state.mixer.lock().await;
        if let Some(mixer) = mixer_lock.as_mut() {
            mixer.remove_participant(&peer_id);
        }
    }

    {
        let mut participants = state.participants.lock().await;
        participants.remove(&peer_id);
    }

    Ok(())
}

#[tauri::command]
async fn get_mixer_status(state: tauri::State<'_, AppState>) -> Result<MixerStatus, String> {
    let mixer_lock = state.mixer.lock().await;
    if let Some(mixer) = mixer_lock.as_ref() {
        let status = mixer.get_status();
        Ok(MixerStatus {
            topology: format!("{:?}", status.topology),
            participant_count: status.participant_count,
            current_mixer: status.current_mixer.map(|s| s.to_string()),
            is_local_mixer: status.is_local_mixer,
            uptime_secs: status.uptime.map(|d| d.as_secs()),
        })
    } else {
        Ok(MixerStatus {
            topology: "NotInitialized".to_string(),
            participant_count: 0,
            current_mixer: None,
            is_local_mixer: false,
            uptime_secs: None,
        })
    }
}

#[tauri::command]
async fn get_room_info(state: tauri::State<'_, AppState>) -> Result<Option<RoomInfo>, String> {
    let room_lock = state.current_room.lock().await;
    Ok(room_lock.as_ref().map(|r| RoomInfo {
        id: r.id.clone(),
        name: r.name.clone(),
        link: r.link.clone(),
        has_password: false,
    }))
}

#[tauri::command]
async fn get_participants(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ParticipantInfo>, String> {
    let participants = state.participants.lock().await;
    Ok(participants.values().cloned().collect())
}

#[tauri::command]
async fn set_muted(state: tauri::State<'_, AppState>, is_muted: bool) -> Result<(), String> {
    let peer_id = {
        let id_lock = state.identity.lock().await;
        id_lock
            .as_ref()
            .map(|i| i.peer_id())
            .ok_or_else(|| "Identity not initialized".to_string())?
    };

    let parsed_peer_id = peer_id
        .parse()
        .map_err(|e| format!("Invalid peer ID format: {}", e))?;

    let cmd_lock = state.network_command.lock().await;
    if let Some(cmd_tx) = cmd_lock.as_ref() {
        let msg = ControlMessage::mute_changed(peer_id.clone(), is_muted);
        cmd_tx
            .send(NetworkCommand::SendControl {
                peer_id: parsed_peer_id,
                message: msg,
            })
            .await
            .map_err(|e| format!("Failed to send mute command: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
async fn export_identity(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let id_lock = state.identity.lock().await;
    let identity = id_lock
        .as_ref()
        .ok_or_else(|| "Identity not initialized".to_string())?;

    let bytes = identity.to_bytes();
    let display_name = identity.display_name().map(|s| s.to_string());
    let peer_id = identity.peer_id();

    let export_data = serde_json::json!({
        "version": 1,
        "peer_id": peer_id,
        "key_bytes": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes),
        "display_name": display_name,
    });

    Ok(export_data.to_string())
}

#[tauri::command]
async fn import_identity(state: tauri::State<'_, AppState>, json: String) -> Result<(), String> {
    let data: serde_json::Value =
        serde_json::from_str(&json).map_err(|e| format!("Invalid JSON: {}", e))?;

    let key_bytes_str = data["key_bytes"]
        .as_str()
        .ok_or_else(|| "Missing key_bytes field".to_string())?;
    let key_bytes =
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, key_bytes_str)
            .map_err(|e| format!("Invalid base64: {}", e))?;

    let mut identity = agora_core::Identity::from_bytes(&key_bytes)
        .map_err(|e| format!("Failed to restore identity: {}", e))?;

    if let Some(name) = data["display_name"].as_str() {
        identity.set_display_name(name.to_string());
    }

    let mut id_lock = state.identity.lock().await;
    *id_lock = Some(identity);

    Ok(())
}

#[tauri::command]
async fn save_settings(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    settings: serde_json::Value,
) -> Result<(), String> {
    let app_settings: AppSettings =
        serde_json::from_value(settings).map_err(|e| format!("Invalid settings: {}", e))?;

    {
        let mut settings_lock = state.settings.lock().await;
        *settings_lock = app_settings.clone();
    }

    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get config dir: {}", e))?;

    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config dir: {}", e))?;

    let settings_path = config_dir.join("settings.json");
    let settings_json = serde_json::to_string_pretty(&app_settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    std::fs::write(&settings_path, settings_json)
        .map_err(|e| format!("Failed to write settings: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn load_settings(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get config dir: {}", e))?;

    let settings_path = config_dir.join("settings.json");

    let settings = if settings_path.exists() {
        let settings_json = std::fs::read_to_string(&settings_path)
            .map_err(|e| format!("Failed to read settings: {}", e))?;
        let app_settings: AppSettings = serde_json::from_str(&settings_json)
            .map_err(|e| format!("Failed to parse settings: {}", e))?;

        {
            let mut settings_lock = state.settings.lock().await;
            *settings_lock = app_settings.clone();
        }

        app_settings
    } else {
        let settings_lock = state.settings.lock().await;
        settings_lock.clone()
    };

    serde_json::to_value(settings).map_err(|e| format!("Failed to serialize settings: {}", e))
}

#[tauri::command]
async fn get_settings(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let settings_lock = state.settings.lock().await;
    let settings = settings_lock.clone();

    serde_json::to_value(settings).map_err(|e| format!("Failed to serialize settings: {}", e))
}

#[tauri::command]
async fn get_network_info(state: tauri::State<'_, AppState>) -> Result<NetworkInfo, String> {
    let id_lock = state.identity.lock().await;
    let peer_id = id_lock
        .as_ref()
        .map(|i| i.peer_id())
        .ok_or_else(|| "Identity not initialized".to_string())?;
    drop(id_lock);

    let listen_addrs = state.listen_addrs.lock().await.clone();
    let connected_peers = state.connected_peers.lock().await.clone();

    Ok(NetworkInfo {
        peer_id,
        listen_addrs,
        connected_peers,
    })
}

#[tauri::command]
async fn connect_peer(state: tauri::State<'_, AppState>, addr: String) -> Result<(), String> {
    tracing::info!("[NETWORK] Connecting to peer: {}", addr);

    let multiaddr: agora_core::Multiaddr = addr
        .parse()
        .map_err(|e| format!("Invalid address format: {}", e))?;

    let cmd_lock = state.network_command.lock().await;
    if let Some(cmd_tx) = cmd_lock.as_ref() {
        cmd_tx
            .send(NetworkCommand::ConnectToPeer { addr: multiaddr })
            .await
            .map_err(|e| format!("Failed to connect: network unavailable - {}", e))?;
        tracing::info!("[NETWORK] Connection request sent");
        Ok(())
    } else {
        Err("Network not started. Please start the network first.".to_string())
    }
}

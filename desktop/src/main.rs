#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use agora_core::{Identity, NetworkNode, Room, RoomConfig};
use std::sync::Arc;
use tokio::sync::Mutex;

struct AppState {
    identity: Arc<Mutex<Option<Identity>>>,
    network: Arc<Mutex<Option<NetworkNode>>>,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    
    let state = AppState {
        identity: Arc::new(Mutex::new(None)),
        network: Arc::new(Mutex::new(None)),
    };
    
    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            init_identity,
            get_peer_id,
            get_display_name,
            set_display_name,
            create_room,
            join_room,
            start_network,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
async fn init_identity(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let identity = Identity::generate()
        .map_err(|e| format!("Failed to generate identity: {}", e))?;
    let peer_id = identity.peer_id();
    
    let mut id_lock = state.identity.lock().await;
    *id_lock = Some(identity);
    
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
async fn set_display_name(
    state: tauri::State<'_, AppState>,
    name: String,
) -> Result<(), String> {
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
) -> Result<String, String> {
    let id_lock = state.identity.lock().await;
    let peer_id = id_lock
        .as_ref()
        .map(|i| i.peer_id())
        .ok_or_else(|| "Identity not initialized".to_string())?;
    drop(id_lock);
    
    let config = RoomConfig {
        name,
        password,
        max_participants: Some(20),
    };
    
    let room = Room::new(peer_id, config);
    let link = room.share_link();
    
    Ok(link)
}

#[tauri::command]
async fn join_room(room_link: String) -> Result<String, String> {
    let (room_id, _password) = agora_core::room::parse_room_link(&room_link)
        .ok_or_else(|| "Invalid room link".to_string())?;
    
    Ok(room_id)
}

#[tauri::command]
async fn start_network(
    state: tauri::State<'_, AppState>,
    listen_port: Option<u16>,
) -> Result<String, String> {
    let listen_addr = listen_port
        .map(|p| format!("/ip4/0.0.0.0/tcp/{}", p))
        .unwrap_or_else(|| "/ip4/0.0.0.0/tcp/0".to_string());
    
    let network = NetworkNode::new(Some(&listen_addr))
        .await
        .map_err(|e| format!("Failed to start network: {}", e))?;
    
    let peer_id = network.peer_id_string();
    
    let mut net_lock = state.network.lock().await;
    *net_lock = Some(network);
    
    Ok(peer_id)
}

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::sync::OnceLock;
use tokio::runtime::Runtime;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

#[allow(dead_code)]
fn runtime() -> Option<&'static Runtime> {
    RUNTIME.get()
}

fn init_runtime() -> Option<&'static Runtime> {
    RUNTIME.get_or_init(|| Runtime::new().expect("Failed to create tokio runtime"));
    RUNTIME.get()
}

fn to_c_string(s: impl Into<String>) -> *mut c_char {
    match CString::new(s.into()) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => {
            let fallback = CString::new("ERROR:Internal error - invalid string")
                .unwrap_or_else(|_| CString::new("ERROR").unwrap());
            fallback.into_raw()
        }
    }
}

fn error_c_string(msg: &str) -> *mut c_char {
    let error_msg = format!("ERROR:{}", msg);
    to_c_string(error_msg)
}

/// Initialize the Agora library and generate a new identity.
/// Returns a null-terminated string containing the peer ID.
/// Caller must free the returned string with `agora_free_string`.
#[no_mangle]
pub extern "C" fn agora_init() -> *mut c_char {
    let Some(rt) = init_runtime() else {
        return error_c_string("Failed to create runtime");
    };

    let result: Result<String, String> = rt.block_on(async {
        let identity = crate::Identity::generate().map_err(|e| e.to_string())?;
        Ok(identity.peer_id())
    });

    match result {
        Ok(peer_id) => to_c_string(peer_id),
        Err(e) => error_c_string(&e),
    }
}

/// Free a string returned by Agora FFI functions.
///
/// # Safety
/// - `s` must be a valid pointer returned by an Agora FFI function, or null.
/// - `s` must not have been freed already.
/// - `s` must not be used after this call.
#[no_mangle]
pub unsafe extern "C" fn agora_free_string(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

/// Generate a new random room ID.
/// Caller must free the returned string with `agora_free_string`.
#[no_mangle]
pub extern "C" fn agora_generate_room_id() -> *mut c_char {
    let room_id = crate::room::generate_room_id();
    to_c_string(room_id)
}

/// Create an agora:// room link from a room ID.
///
/// # Safety
/// - `room_id` must be a valid null-terminated C string.
/// - Caller must free the returned string with `agora_free_string`.
#[no_mangle]
pub unsafe extern "C" fn agora_create_room_link(room_id: *const c_char) -> *mut c_char {
    if room_id.is_null() {
        return error_c_string("Null room_id pointer");
    }

    let room_id_str = match CStr::from_ptr(room_id).to_str() {
        Ok(s) => s,
        Err(_) => return error_c_string("Invalid UTF-8 in room_id"),
    };

    let link = format!("agora://room/{}", room_id_str);
    to_c_string(link)
}

/// Parse an agora:// room link and extract room ID and optional password.
/// Returns "room_id:password" or "room_id" if no password.
///
/// # Safety
/// - `link` must be a valid null-terminated C string.
/// - Caller must free the returned string with `agora_free_string`.
#[no_mangle]
pub unsafe extern "C" fn agora_parse_room_link(link: *const c_char) -> *mut c_char {
    if link.is_null() {
        return error_c_string("Null link pointer");
    }

    let link_str = match CStr::from_ptr(link).to_str() {
        Ok(s) => s,
        Err(_) => return error_c_string("Invalid UTF-8 in link"),
    };

    match crate::room::parse_room_link(link_str) {
        Some((room_id, password)) => {
            let result = if let Some(pwd) = password {
                format!("{}:{}", room_id, pwd)
            } else {
                room_id
            };
            to_c_string(result)
        }
        None => error_c_string("Invalid link format"),
    }
}

#[repr(C)]
pub struct AgoraAudioDevice {
    pub name: *mut c_char,
    pub is_default: bool,
    pub channels: u16,
    pub sample_rate: u32,
}

/// Get list of available audio devices.
///
/// # Safety
/// - `out_devices` must be a valid pointer to a pointer.
/// - `out_count` must be a valid pointer to a usize.
/// - Caller must free the returned devices with `agora_free_audio_devices`.
#[no_mangle]
pub unsafe extern "C" fn agora_get_audio_devices(
    out_devices: *mut *mut AgoraAudioDevice,
    out_count: *mut usize,
) {
    if out_devices.is_null() || out_count.is_null() {
        return;
    }

    let input_devices = crate::AudioDevice::input_devices().unwrap_or_default();
    let output_devices = crate::AudioDevice::output_devices().unwrap_or_default();

    let mut devices: Vec<AgoraAudioDevice> = Vec::new();

    for d in input_devices {
        if let Ok(name) = CString::new(d.name) {
            devices.push(AgoraAudioDevice {
                name: name.into_raw(),
                is_default: d.is_default,
                channels: d.channels,
                sample_rate: d.sample_rate,
            });
        }
    }

    for d in output_devices {
        if let Ok(name) = CString::new(d.name) {
            devices.push(AgoraAudioDevice {
                name: name.into_raw(),
                is_default: d.is_default,
                channels: d.channels,
                sample_rate: d.sample_rate,
            });
        }
    }

    let count = devices.len();
    if count == 0 {
        *out_count = 0;
        *out_devices = ptr::null_mut();
        return;
    }

    let layout = match std::alloc::Layout::array::<AgoraAudioDevice>(count) {
        Ok(l) => l,
        Err(_) => {
            *out_count = 0;
            *out_devices = ptr::null_mut();
            return;
        }
    };

    let ptr = std::alloc::alloc_zeroed(layout) as *mut AgoraAudioDevice;
    if ptr.is_null() {
        for device in devices {
            if !device.name.is_null() {
                drop(CString::from_raw(device.name));
            }
        }
        *out_count = 0;
        *out_devices = ptr::null_mut();
        return;
    }

    ptr::copy_nonoverlapping(devices.as_ptr(), ptr, count);
    *out_count = count;
    *out_devices = ptr;
}

/// Free audio device list returned by `agora_get_audio_devices`.
///
/// # Safety
/// - `devices` must be a valid pointer returned by `agora_get_audio_devices`, or null.
/// - `count` must be the same count returned by `agora_get_audio_devices`.
/// - Must not be called more than once for the same pointer.
#[no_mangle]
pub unsafe extern "C" fn agora_free_audio_devices(devices: *mut AgoraAudioDevice, count: usize) {
    if devices.is_null() || count == 0 {
        return;
    }

    for i in 0..count {
        let device = &mut *devices.add(i);
        if !device.name.is_null() {
            drop(CString::from_raw(device.name));
        }
    }

    if let Ok(layout) = std::alloc::Layout::array::<AgoraAudioDevice>(count) {
        std::alloc::dealloc(devices as *mut u8, layout);
    }
}

#[repr(C)]
pub struct AgoraNATInfo {
    pub nat_type: *mut c_char,
    pub can_hole_punch: bool,
    pub description: *mut c_char,
}

/// Detect NAT type and hole-punching capability.
/// Returns NAT info that must be freed with `agora_free_nat_info`.
#[no_mangle]
pub extern "C" fn agora_detect_nat() -> AgoraNATInfo {
    let Some(rt) = init_runtime() else {
        return AgoraNATInfo {
            nat_type: error_c_string("No runtime"),
            can_hole_punch: false,
            description: error_c_string("Failed to create runtime"),
        };
    };

    let result: Result<crate::NatType, String> = rt.block_on(async {
        let mut nat = crate::NatTraversal::new(None);
        nat.detect_nat_type().await.map_err(|e| e.to_string())
    });

    match result {
        Ok(nat) => {
            let type_str = format!("{:?}", nat);
            AgoraNATInfo {
                nat_type: to_c_string(type_str),
                can_hole_punch: nat.can_hole_punch(),
                description: to_c_string(nat.description()),
            }
        }
        Err(e) => AgoraNATInfo {
            nat_type: to_c_string("Unknown"),
            can_hole_punch: false,
            description: to_c_string(e),
        },
    }
}

/// Free NAT info returned by `agora_detect_nat`.
///
/// # Safety
/// - `info` must be a valid `AgoraNATInfo` returned by `agora_detect_nat`.
/// - Must not be called more than once for the same info.
#[no_mangle]
pub extern "C" fn agora_free_nat_info(info: AgoraNATInfo) {
    unsafe {
        if !info.nat_type.is_null() {
            drop(CString::from_raw(info.nat_type));
        }
        if !info.description.is_null() {
            drop(CString::from_raw(info.description));
        }
    }
}

#[repr(C)]
pub struct AgoraMixerInfo {
    pub topology: *mut c_char,
    pub participant_count: usize,
    pub is_mixer: bool,
}

#[no_mangle]
pub extern "C" fn agora_test_mixer(participants: usize) -> AgoraMixerInfo {
    let mut mixer = crate::MixerManager::new("local_peer".to_string(), None);

    for i in 0..participants {
        mixer.add_participant(format!("peer_{}", i));
    }

    let status = mixer.get_status();

    AgoraMixerInfo {
        topology: to_c_string(format!("{:?}", status.topology)),
        participant_count: status.participant_count,
        is_mixer: status.is_local_mixer,
    }
}

#[no_mangle]
pub extern "C" fn agora_free_mixer_info(info: AgoraMixerInfo) {
    unsafe {
        if !info.topology.is_null() {
            drop(CString::from_raw(info.topology));
        }
    }
}

#[no_mangle]
pub extern "C" fn agora_version() -> *mut c_char {
    to_c_string(env!("CARGO_PKG_VERSION").to_string())
}

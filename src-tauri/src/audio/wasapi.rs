use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use windows::core::{Interface, PCWSTR, PWSTR};
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Foundation::{CloseHandle, BOOL, RPC_E_CHANGED_MODE};
use windows::Win32::Media::Audio::Endpoints::{IAudioEndpointVolume, IAudioMeterInformation};
use windows::Win32::Media::Audio::{
    eCapture, eConsole, eRender, AudioSessionStateActive, AudioSessionStateExpired,
    IAudioSessionControl2, IAudioSessionEnumerator, IAudioSessionManager2, IMMDevice,
    IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
};
use windows::Win32::System::Com::StructuredStorage::PropVariantClear;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL,
    COINIT_MULTITHREADED, STGM_READ,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};

use super::models::{AudioDeviceInfo, AudioSessionInfo, AudioSnapshot, MicInfo};
use super::policy_config::set_default_audio_endpoint;

pub const SYSTEM_SOUNDS_KEY: &str = "__system_sounds__";

struct ComGuard {
    should_uninit: bool,
}

impl ComGuard {
    fn new() -> Result<Self, String> {
        unsafe {
            let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
            if hr.is_ok() {
                Ok(Self { should_uninit: true })
            } else if hr == RPC_E_CHANGED_MODE {
                Ok(Self {
                    should_uninit: false,
                })
            } else {
                Err(format!("CoInitializeEx failed: {hr:?}"))
            }
        }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.should_uninit {
            unsafe {
                CoUninitialize();
            }
        }
    }
}

unsafe fn create_enumerator() -> Result<IMMDeviceEnumerator, String> {
    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
        .map_err(|e| format!("Failed to create MMDeviceEnumerator: {e}"))
}

unsafe fn get_device_id(device: &IMMDevice) -> Result<String, String> {
    let pwstr = device
        .GetId()
        .map_err(|e| format!("IMMDevice::GetId failed: {e}"))?;
    let id = pwstr.to_string().unwrap_or_default();
    CoTaskMemFree(Some(pwstr.0 as *const _));
    Ok(id)
}

unsafe fn get_device_friendly_name(device: &IMMDevice) -> String {
    let Ok(store) = device.OpenPropertyStore(STGM_READ) else {
        return "Audio Device".to_string();
    };
    let Ok(mut prop) = store.GetValue(&PKEY_Device_FriendlyName) else {
        return "Audio Device".to_string();
    };
    let name = prop.to_string();
    let _ = PropVariantClear(&mut prop);
    if name.trim().is_empty() {
        "Audio Device".to_string()
    } else {
        name
    }
}

fn classify_device_kind(name: &str) -> String {
    let lower = name.to_lowercase();
    if lower.contains("headphone")
        || lower.contains("headset")
        || lower.contains("kulakl")
        || lower.contains("earphone")
        || lower.contains("earbud")
        || lower.contains("airpod")
        || lower.contains("buds")
        || lower.contains("hyperx")
        || lower.contains("arctis")
        || lower.contains("razer")
        || lower.contains("logitech g")
        || lower.contains("corsair")
        || lower.contains("cloud")
        || lower.contains("dt 770")
        || lower.contains("dt 990")
        || lower.contains("sennheiser")
        || lower.contains("ath-")
    {
        "headphones".to_string()
    } else if lower.contains("hdmi")
        || lower.contains("display")
        || lower.contains("monitor")
        || lower.contains("nvidia")
        || lower.contains("amd high definition")
        || lower.contains("lg ")
        || lower.contains("asus ")
        || lower.contains("samsung")
        || lower.contains("benq")
    {
        "monitor".to_string()
    } else if lower.contains("speaker")
        || lower.contains("hoparl")
        || lower.contains("realtek")
        || lower.contains("soundbar")
        || lower.contains("bose")
        || lower.contains("jbl")
        || lower.contains("edifier")
    {
        "speaker".to_string()
    } else {
        "generic".to_string()
    }
}

unsafe fn get_device_volume_and_peak(device: &IMMDevice) -> (f32, bool, f32) {
    let (volume, muted) = match device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) {
        Ok(ep) => {
            let v = ep.GetMasterVolumeLevelScalar().unwrap_or(0.0).clamp(0.0, 1.0);
            let m = ep.GetMute().map(|b| b.as_bool()).unwrap_or(false);
            (v, m)
        }
        Err(_) => (0.0, false),
    };

    let peak = match device.Activate::<IAudioMeterInformation>(CLSCTX_ALL, None) {
        Ok(meter) => meter.GetPeakValue().unwrap_or(0.0).clamp(0.0, 1.0),
        Err(_) => 0.0,
    };

    (volume, muted, peak)
}

fn get_process_exe_name(pid: u32) -> Option<String> {
    if pid == 0 {
        return None;
    }
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = vec![0u16; 1024];
        let mut size = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR::from_raw(buf.as_mut_ptr()),
            &mut size,
        )
        .is_ok();
        let _ = CloseHandle(handle);

        if !ok || size == 0 {
            return None;
        }
        let path_str = String::from_utf16_lossy(&buf[..size as usize]);
        Path::new(&path_str)
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    }
}

fn prettify_app_name(exe_name: &str) -> String {
    let stem = exe_name
        .strip_suffix(".exe")
        .or_else(|| exe_name.strip_suffix(".EXE"))
        .unwrap_or(exe_name);

    match stem.to_lowercase().as_str() {
        "chrome" => "Google Chrome".to_string(),
        "msedge" => "Microsoft Edge".to_string(),
        "firefox" => "Mozilla Firefox".to_string(),
        "brave" => "Brave Browser".to_string(),
        "opera" => "Opera".to_string(),
        "discord" | "discordptb" | "discordcanary" => "Discord".to_string(),
        "spotify" => "Spotify".to_string(),
        "obs64" | "obs32" => "OBS Studio".to_string(),
        "vlc" => "VLC Media Player".to_string(),
        "steam" | "steamwebhelper" => "Steam".to_string(),
        "valorant-win64-shipping" => "VALORANT".to_string(),
        "cs2" => "Counter-Strike 2".to_string(),
        "javaw" => "Minecraft / Java".to_string(),
        "telegram" => "Telegram".to_string(),
        "whatsapp" => "WhatsApp".to_string(),
        "teams" | "ms-teams" => "Microsoft Teams".to_string(),
        "zoom" => "Zoom".to_string(),
        "code" => "VS Code".to_string(),
        _ => {
            let mut chars = stem.chars();
            match chars.next() {
                None => stem.to_string(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        }
    }
}

fn session_app_key_and_names(control2: &IAudioSessionControl2) -> Option<(String, String, String, u32, bool)> {
    unsafe {
        if control2.IsSystemSoundsSession().is_ok() {
            return Some((
                SYSTEM_SOUNDS_KEY.to_string(),
                "System Sounds".to_string(),
                "system".to_string(),
                0,
                true,
            ));
        }

        let pid = control2.GetProcessId().unwrap_or(0);
        if pid == 0 {
            return None;
        }

        let exe_name = get_process_exe_name(pid)?;
        let app_key = exe_name.to_lowercase();

        // Try COM display name first if it's not a resource string (@...)
        let com_display = control2
            .GetDisplayName()
            .ok()
            .map(|p| {
                let s = p.to_string().unwrap_or_default();
                CoTaskMemFree(Some(p.0 as *const _));
                s
            })
            .unwrap_or_default();

        let display_name = if !com_display.trim().is_empty() && !com_display.trim().starts_with('@') {
            com_display.trim().to_string()
        } else {
            prettify_app_name(&exe_name)
        };

        Some((app_key, display_name, exe_name, pid, false))
    }
}

pub fn get_audio_snapshot(ducked_apps: &HashMap<String, f32>) -> Result<AudioSnapshot, String> {
    let _com = ComGuard::new()?;
    unsafe {
        let enumerator = create_enumerator()?;

        // Default output device
        let default_output = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok();
        let default_output_id = default_output
            .as_ref()
            .and_then(|d| get_device_id(d).ok());

        // Enumerate all active render devices
        let mut output_devices = Vec::new();
        if let Ok(collection) = enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) {
            let count = collection.GetCount().unwrap_or(0);
            for i in 0..count {
                if let Ok(device) = collection.Item(i) {
                    if let Ok(id) = get_device_id(&device) {
                        let name = get_device_friendly_name(&device);
                        let is_default = default_output_id.as_deref() == Some(id.as_str());
                        let (volume, muted, peak) = get_device_volume_and_peak(&device);
                        let kind = classify_device_kind(&name);
                        output_devices.push(AudioDeviceInfo {
                            id,
                            name,
                            is_default,
                            volume,
                            muted,
                            peak,
                            kind,
                        });
                    }
                }
            }
        }
        // Sort: default device first, then alphabetically
        output_devices.sort_by(|a, b| b.is_default.cmp(&a.is_default).then(a.name.cmp(&b.name)));

        // Default microphone
        let mic = enumerator
            .GetDefaultAudioEndpoint(eCapture, eConsole)
            .ok()
            .and_then(|dev| {
                let id = get_device_id(&dev).ok()?;
                let name = get_device_friendly_name(&dev);
                let (volume, muted, peak) = get_device_volume_and_peak(&dev);
                Some(MicInfo {
                    id,
                    name,
                    volume,
                    muted,
                    peak,
                })
            });

        // Sessions on default render device
        let mut grouped: BTreeMap<String, AudioSessionInfo> = BTreeMap::new();
        if let Some(ref def_dev) = default_output {
            if let Ok(manager) = def_dev.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) {
                if let Ok(session_enum) = manager.GetSessionEnumerator() {
                    collect_sessions(&session_enum, ducked_apps, &mut grouped);
                }
            }
        }

        let mut sessions: Vec<AudioSessionInfo> = grouped.into_values().collect();
        // Sort: active first, then non-system-sounds before system sounds, then display_name
        sessions.sort_by(|a, b| {
            a.is_system_sounds
                .cmp(&b.is_system_sounds)
                .then(b.is_active.cmp(&a.is_active))
                .then(a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()))
        });

        Ok(AudioSnapshot {
            output_devices,
            default_output_id,
            mic,
            sessions,
        })
    }
}

unsafe fn collect_sessions(
    session_enum: &IAudioSessionEnumerator,
    ducked_apps: &HashMap<String, f32>,
    grouped: &mut BTreeMap<String, AudioSessionInfo>,
) {
    let count = session_enum.GetCount().unwrap_or(0);
    for i in 0..count {
        let Ok(control) = session_enum.GetSession(i) else {
            continue;
        };
        let state = control.GetState().unwrap_or(AudioSessionStateExpired);
        if state == AudioSessionStateExpired {
            continue;
        }
        let is_active = state == AudioSessionStateActive;

        let Ok(control2) = control.cast::<IAudioSessionControl2>() else {
            continue;
        };

        let Some((app_key, display_name, exe_name, pid, is_system_sounds)) =
            session_app_key_and_names(&control2)
        else {
            continue;
        };

        let (volume, muted) = match control.cast::<ISimpleAudioVolume>() {
            Ok(vol) => {
                let v = vol.GetMasterVolume().unwrap_or(1.0).clamp(0.0, 1.0);
                let m = vol.GetMute().map(|b| b.as_bool()).unwrap_or(false);
                (v, m)
            }
            Err(_) => (1.0, false),
        };

        let peak = match control.cast::<IAudioMeterInformation>() {
            Ok(meter) => meter.GetPeakValue().unwrap_or(0.0).clamp(0.0, 1.0),
            Err(_) => 0.0,
        };

        let is_ducked = ducked_apps.contains_key(&app_key);

        grouped
            .entry(app_key.clone())
            .and_modify(|existing| {
                if peak > existing.peak {
                    existing.peak = peak;
                }
                if is_active {
                    existing.is_active = true;
                    existing.volume = volume;
                    existing.muted = muted;
                }
            })
            .or_insert(AudioSessionInfo {
                app_key,
                display_name,
                exe_name,
                pid,
                volume,
                muted,
                peak,
                is_active,
                is_system_sounds,
                is_ducked,
            });
    }
}

unsafe fn find_device_by_id_or_default(
    enumerator: &IMMDeviceEnumerator,
    device_id: Option<&str>,
) -> Result<IMMDevice, String> {
    if let Some(id) = device_id {
        let wide: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();
        enumerator
            .GetDevice(PCWSTR::from_raw(wide.as_ptr()))
            .map_err(|e| format!("Device not found ({id}): {e}"))
    } else {
        enumerator
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .map_err(|e| format!("Default render device not found: {e}"))
    }
}

pub fn set_output_volume(device_id: Option<&str>, volume: f32) -> Result<(), String> {
    let _com = ComGuard::new()?;
    let clamped = volume.clamp(0.0, 1.0);
    unsafe {
        let enumerator = create_enumerator()?;
        let device = find_device_by_id_or_default(&enumerator, device_id)?;
        let ep = device
            .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
            .map_err(|e| format!("Activate IAudioEndpointVolume failed: {e}"))?;
        ep.SetMasterVolumeLevelScalar(clamped, std::ptr::null())
            .map_err(|e| format!("SetMasterVolumeLevelScalar failed: {e}"))?;
        Ok(())
    }
}

pub fn set_output_mute(device_id: Option<&str>, muted: bool) -> Result<(), String> {
    let _com = ComGuard::new()?;
    unsafe {
        let enumerator = create_enumerator()?;
        let device = find_device_by_id_or_default(&enumerator, device_id)?;
        let ep = device
            .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
            .map_err(|e| format!("Activate IAudioEndpointVolume failed: {e}"))?;
        ep.SetMute(BOOL::from(muted), std::ptr::null())
            .map_err(|e| format!("SetMute failed: {e}"))?;
        Ok(())
    }
}

pub fn set_default_output_device(device_id: &str) -> Result<String, String> {
    let _com = ComGuard::new()?;
    unsafe {
        set_default_audio_endpoint(device_id)?;
        let enumerator = create_enumerator()?;
        let device = find_device_by_id_or_default(&enumerator, Some(device_id))?;
        Ok(get_device_friendly_name(&device))
    }
}

pub fn set_mic_volume(volume: f32) -> Result<(), String> {
    let _com = ComGuard::new()?;
    let clamped = volume.clamp(0.0, 1.0);
    unsafe {
        let enumerator = create_enumerator()?;
        let device = enumerator
            .GetDefaultAudioEndpoint(eCapture, eConsole)
            .map_err(|e| format!("No default microphone: {e}"))?;
        let ep = device
            .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
            .map_err(|e| format!("Activate mic endpoint failed: {e}"))?;
        ep.SetMasterVolumeLevelScalar(clamped, std::ptr::null())
            .map_err(|e| format!("Set mic volume failed: {e}"))?;
        Ok(())
    }
}

pub fn set_mic_mute(muted: bool) -> Result<(), String> {
    let _com = ComGuard::new()?;
    unsafe {
        let enumerator = create_enumerator()?;
        let device = enumerator
            .GetDefaultAudioEndpoint(eCapture, eConsole)
            .map_err(|e| format!("No default microphone: {e}"))?;
        let ep = device
            .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
            .map_err(|e| format!("Activate mic endpoint failed: {e}"))?;
        ep.SetMute(BOOL::from(muted), std::ptr::null())
            .map_err(|e| format!("Set mic mute failed: {e}"))?;
        Ok(())
    }
}

pub fn toggle_mic_mute() -> Result<(String, bool), String> {
    let _com = ComGuard::new()?;
    unsafe {
        let enumerator = create_enumerator()?;
        let device = enumerator
            .GetDefaultAudioEndpoint(eCapture, eConsole)
            .map_err(|e| format!("No default microphone: {e}"))?;
        let name = get_device_friendly_name(&device);
        let ep = device
            .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
            .map_err(|e| format!("Activate mic endpoint failed: {e}"))?;
        let current = ep.GetMute().map(|b| b.as_bool()).unwrap_or(false);
        let next = !current;
        ep.SetMute(BOOL::from(next), std::ptr::null())
            .map_err(|e| format!("Toggle mic mute failed: {e}"))?;
        Ok((name, next))
    }
}

unsafe fn for_each_matching_session<F>(app_key: &str, mut f: F) -> Result<Option<String>, String>
where
    F: FnMut(&ISimpleAudioVolume),
{
    let target = app_key.to_lowercase();
    let enumerator = create_enumerator()?;
    let def_dev = enumerator
        .GetDefaultAudioEndpoint(eRender, eConsole)
        .map_err(|e| format!("Default output device unavailable: {e}"))?;
    let manager = def_dev
        .Activate::<IAudioSessionManager2>(CLSCTX_ALL, None)
        .map_err(|e| format!("Activate IAudioSessionManager2 failed: {e}"))?;
    let session_enum = manager
        .GetSessionEnumerator()
        .map_err(|e| format!("GetSessionEnumerator failed: {e}"))?;

    let count = session_enum.GetCount().unwrap_or(0);
    let mut matched_display: Option<String> = None;

    for i in 0..count {
        let Ok(control) = session_enum.GetSession(i) else {
            continue;
        };
        let Ok(control2) = control.cast::<IAudioSessionControl2>() else {
            continue;
        };
        let Some((key, display_name, _, _, _)) = session_app_key_and_names(&control2) else {
            continue;
        };
        if key == target {
            if let Ok(vol) = control.cast::<ISimpleAudioVolume>() {
                f(&vol);
                if matched_display.is_none() {
                    matched_display = Some(display_name);
                }
            }
        }
    }

    Ok(matched_display)
}

pub fn set_app_volume(app_key: &str, volume: f32) -> Result<(), String> {
    let _com = ComGuard::new()?;
    let clamped = volume.clamp(0.0, 1.0);
    unsafe {
        for_each_matching_session(app_key, |vol| {
            let _ = vol.SetMasterVolume(clamped, std::ptr::null());
        })?;
        Ok(())
    }
}

pub fn set_app_mute(app_key: &str, muted: bool) -> Result<(), String> {
    let _com = ComGuard::new()?;
    unsafe {
        for_each_matching_session(app_key, |vol| {
            let _ = vol.SetMute(BOOL::from(muted), std::ptr::null());
        })?;
        Ok(())
    }
}

pub fn toggle_app_mute(app_key: &str) -> Result<Option<(String, bool)>, String> {
    let _com = ComGuard::new()?;
    unsafe {
        let mut next_mute: Option<bool> = None;
        let display = for_each_matching_session(app_key, |vol| {
            let target = match next_mute {
                Some(m) => m,
                None => {
                    let cur = vol.GetMute().map(|b| b.as_bool()).unwrap_or(false);
                    let m = !cur;
                    next_mute = Some(m);
                    m
                }
            };
            let _ = vol.SetMute(BOOL::from(target), std::ptr::null());
        })?;
        Ok(display.zip(next_mute))
    }
}

pub fn toggle_app_duck(
    app_key: &str,
    duck_level: f32,
    ducked_apps: &mut HashMap<String, f32>,
) -> Result<Option<(String, bool, f32)>, String> {
    let _com = ComGuard::new()?;
    let key = app_key.to_lowercase();
    let duck_clamped = duck_level.clamp(0.0, 1.0);

    unsafe {
        if let Some(prev_vol) = ducked_apps.remove(&key) {
            let display = for_each_matching_session(&key, |vol| {
                let _ = vol.SetMasterVolume(prev_vol, std::ptr::null());
            })?;
            Ok(display.map(|d| (d, false, prev_vol)))
        } else {
            let mut captured_vol: Option<f32> = None;
            let display = for_each_matching_session(&key, |vol| {
                if captured_vol.is_none() {
                    captured_vol = Some(vol.GetMasterVolume().unwrap_or(1.0).clamp(0.0, 1.0));
                }
                let _ = vol.SetMasterVolume(duck_clamped, std::ptr::null());
            })?;
            if let Some(d) = display {
                let saved = captured_vol.unwrap_or(1.0);
                ducked_apps.insert(key, saved);
                Ok(Some((d, true, duck_clamped)))
            } else {
                Ok(None)
            }
        }
    }
}

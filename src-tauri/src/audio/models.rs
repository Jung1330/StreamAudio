use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub volume: f32,
    pub muted: bool,
    pub peak: f32,
    pub kind: String, // "headphones" | "speaker" | "monitor" | "generic"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MicInfo {
    pub id: String,
    pub name: String,
    pub volume: f32,
    pub muted: bool,
    pub peak: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSessionInfo {
    pub app_key: String,
    pub display_name: String,
    pub exe_name: String,
    pub pid: u32,
    pub volume: f32,
    pub muted: bool,
    pub peak: f32,
    pub is_active: bool,
    pub is_system_sounds: bool,
    pub is_ducked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSnapshot {
    pub output_devices: Vec<AudioDeviceInfo>,
    pub default_output_id: Option<String>,
    pub mic: Option<MicInfo>,
    pub sessions: Vec<AudioSessionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppShortcutRule {
    pub id: String,
    pub app_key: String,
    pub app_name: String,
    pub shortcut: String,
    pub mode: String, // "mute" | "duck"
    pub duck_percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub language: String, // "tr" | "en"
    pub pinned: bool,
    pub switch_shortcut: String,
    pub mic_mute_shortcut: String,
    pub favorite_device_a: Option<String>,
    pub favorite_device_b: Option<String>,
    pub app_shortcuts: Vec<AppShortcutRule>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            language: "tr".to_string(),
            pinned: false,
            switch_shortcut: "Ctrl+Alt+S".to_string(),
            mic_mute_shortcut: "Ctrl+Alt+M".to_string(),
            favorite_device_a: None,
            favorite_device_b: None,
            app_shortcuts: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OsdToastPayload {
    pub title: String,
    pub subtitle: String,
    pub icon: String, // "headphones" | "speaker" | "mic-off" | "mic-on" | "mute" | "duck" | "volume"
}

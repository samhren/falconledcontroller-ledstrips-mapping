use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GlobalEffect {
    pub kind: String,                   // "Solid" | "Rainbow"
    pub params: HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GlobalEffectConfig {
    pub effect: GlobalEffect,
    pub targets: Option<Vec<u64>>, // List of Strip IDs. None or Empty implies "All"
}

impl Default for GlobalEffect {
    fn default() -> Self {
        Self { kind: "Rainbow".into(), params: HashMap::new() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Scene {
    pub id: u64,
    pub name: String,
    pub kind: String,                  // "Masks" | "Global"
    #[serde(default = "default_category")]
    pub category: String,              // User-defined category for organization
    #[serde(default)]
    pub masks: Vec<Mask>,              // used when kind=="Masks"
    pub global: Option<GlobalEffect>,  // DEPRECATED: used only for migration
    #[serde(default)]
    pub global_effects: Vec<GlobalEffectConfig>, // used when kind=="Global"
    #[serde(default)]
    pub launchpad_btn: Option<u8>,
    #[serde(default)]
    pub launchpad_is_cc: bool,
    #[serde(default)]
    pub launchpad_color: Option<u8>,
}

fn default_category() -> String {
    "Uncategorized".to_string()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PixelStrip {
    pub id: u64,
    pub universe: u16,
    pub start_channel: u16,
    pub pixel_count: usize,
    pub x: f32, // Normalized 0..1
    pub y: f32, // Normalized 0..1
    pub spacing: f32, // Relative spacing 0..1
    #[serde(default)]
    pub flipped: bool, // true = 180 deg (Left), false = 0 deg (Right)
    #[serde(default = "default_color_order")]
    pub color_order: String, // "RGB", "GRB", "BGR"
    #[serde(skip)]
    pub data: Vec<[u8; 3]>, // RGB Data
}

fn default_color_order() -> String {
    "RGB".to_string()
}

impl Default for PixelStrip {
    fn default() -> Self {
        Self {
            id: 0,
            universe: 1,
            start_channel: 1,
            pixel_count: 50,
            x: 0.5,
            y: 0.5,
            spacing: 0.05,
            flipped: false,
            color_order: "RGB".to_string(),
            data: vec![[0, 0, 0]; 50],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Mask {
    pub id: u64,
    pub mask_type: String, // "scanner", "radial"
    pub x: f32,
    pub y: f32,
    pub params: HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NetworkConfig {
    pub use_multicast: bool,
    pub unicast_ip: String,
    pub universe: u16,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            use_multicast: true,
            unicast_ip: "192.168.1.50".to_string(), // Default placeholder
            universe: 1,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AudioConfig {
    pub latency_ms: f32,
    pub use_flywheel: bool,
    pub hybrid_sync: bool,
    pub sensitivity: f32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            latency_ms: 0.0,
            use_flywheel: true,
            hybrid_sync: false,
            sensitivity: 0.5,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct AppState {
    pub strips: Vec<PixelStrip>,
    pub masks: Vec<Mask>,
    #[serde(default)]
    pub scenes: Vec<Scene>,
    pub selected_scene_id: Option<u64>,
    #[serde(default)]
    pub network: NetworkConfig,
    pub bind_address: Option<String>,
    pub mode: String, // "global", "spatial"
    pub effect: String,
    #[serde(default)]
    pub audio: AudioConfig,
    #[serde(default)]
    pub layout_locked: bool,
    #[serde(default = "default_midi_enabled")]
    pub midi_enabled: bool,
}

fn default_midi_enabled() -> bool {
    true
}

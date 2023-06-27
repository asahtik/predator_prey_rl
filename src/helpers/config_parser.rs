use serde::Deserialize;
use std::env;
use std::fs;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub world: WorldConfig,
    pub camera: CameraConfig,
    pub rl: RLConfig,
    pub predator: AgentConfig,
    pub prey: AgentConfig,
}

#[derive(Deserialize, Debug)]
pub struct WorldConfig {
    pub world_width: f32,
    pub world_height: f32,
    pub map: String,
    pub water_multiplier: f32,
    pub forrest_vision_multiplier: f32,
    pub batch_spawn_count: u32,
    pub batch_spawn_radius: f32,
}

#[derive(Deserialize, Debug)]
pub struct CameraConfig {
    pub default_radius: f32,
    pub translate_mouse_sensitivity: f32,
    pub rotate_mouse_sensitivity: f32,
    pub scroll_sensitivity: f32,
}

#[derive(Deserialize, Debug)]
pub struct RLConfig {
    pub learn: bool,
    pub replay_buffer_size: usize,
    pub layers: Vec<usize>,
    pub learning_rate: f32,
    pub eps_step: f32,
    pub eps_min: f32,
    pub discount: f32,
    pub sample_count: usize,
    pub batch_size: usize,
    pub frames_per_update: usize,
    pub updates_per_target: usize,
    pub num_updates: usize,
    pub updates_per_save: usize,
    pub updates_per_reset: usize,
    pub updates_per_swap: Option<usize>,
    pub save_path: String,
    pub load_path: Option<String>,
    pub load_model_name: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct AgentConfig {
    pub count: u32,
    pub size: f32,
    pub wl_ratio: f32,
    pub hl_ratio: f32,
    pub walk_speed: f32,
    pub walk_acceleration: f32,
    pub run_speed: f32,
    pub run_acceleration: f32,
    pub deceleration: f32,
    pub turn_speed: f32,
    pub vision_range: f32,
    pub vision_fov: f32,
    pub vision_rays: usize,
    pub hearing_range: f32,
    pub hearing_rays: usize,
    pub food_quantity: u32,
    pub eating_speed: f32,
    pub procreation_min_energy: f32,
    pub procreation_attempt_energy_loss: f32,
    pub procreation_energy_loss: f32,
    pub tick_energy_loss: f32,
    pub turn_energy_loss: f32,
    pub walk_energy_loss: f32,
    pub run_energy_loss: f32,
    pub attack_energy_loss: f32, // Predator only
    pub life: usize,
    pub rewards: RewardsConfig,
}

#[derive(Deserialize, Debug)]
pub struct RewardsConfig {
    pub tick: f32,
    pub turn: f32,
    pub walk: f32,
    pub run: f32,
    pub eat: f32,
    pub procreation: f32,
    pub death: f32,
    pub detecting_prey: f32,
    pub detecting_predator: f32,
    pub detecting_food: f32,
}

pub fn read_config() -> Config {
    let mut path = env::current_dir().unwrap();
    path.push("config.toml");
    if let Ok(config_file) = fs::read_to_string(path.clone()) {
        toml::from_str(config_file.as_str()).expect("Unable to parse toml file")
    } else {
        path.pop();
        path.pop();
        path.push("config.toml");
        let config_file = fs::read_to_string(path).expect("Unable to read config file");
        toml::from_str(config_file.as_str()).expect("Unable to parse toml file")
    }
}

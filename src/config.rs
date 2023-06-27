use bevy::prelude::*;

pub const FPS: f32 = 30.0;
pub const INITIAL_ENERGY: f32 = 80.0;
pub const MAX_ENERGY: f32 = 100.0;

#[derive(Resource)]
pub struct ConfigRes(pub crate::helpers::config_parser::Config);
impl Default for ConfigRes {
    fn default() -> Self {
        Self(crate::helpers::config_parser::read_config())
    }
}

#[derive(Resource, Default)]
pub struct Map {
    pub map: Handle<crate::assets::MapAsset>,
}

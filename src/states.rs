use bevy::prelude::*;

#[derive(States, Clone, Debug, Default, Eq, Hash, PartialEq)]
pub enum AppState {
    #[default]
    MainMenu,
    InGame,
    Paused,
}

#[derive(States, Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum GameState {
    #[default]
    Normal,
    FastForward,
    Skip,
}

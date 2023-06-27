use bevy::{app::PluginGroupBuilder, prelude::*};

mod game_menu;
mod main_menu;
mod pause_menu;

use main_menu::MainMenuPlugin;
use pause_menu::PauseMenuPlugin;

use crate::states::AppState;

pub struct ToggleAppStatePlugin;
impl Plugin for ToggleAppStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(toggle_app_state);
    }
}

pub struct MenuPlugins;
impl PluginGroup for MenuPlugins {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(MainMenuPlugin)
            .add(PauseMenuPlugin)
            .add(ToggleAppStatePlugin)
            .add(game_menu::GameMenuPlugin)
    }
}

fn toggle_app_state(
    state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    match state.0 {
        AppState::MainMenu => {
            if keyboard_input.just_pressed(KeyCode::Space) {
                next_state.set(AppState::InGame);
            }
        }
        AppState::InGame => {
            if keyboard_input.just_pressed(KeyCode::Escape) {
                next_state.set(AppState::Paused);
            }
        }
        AppState::Paused => {
            if keyboard_input.just_pressed(KeyCode::Escape)
                || keyboard_input.just_pressed(KeyCode::Space)
            {
                next_state.set(AppState::InGame);
            }
        }
    }
}

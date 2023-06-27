use bevy::prelude::*;

use crate::states::AppState;

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(spawn_main_menu.in_schedule(OnEnter(AppState::MainMenu)))
            .add_system(despawn_main_menu.in_schedule(OnExit(AppState::MainMenu)));
    }
}

#[derive(Component)]
struct MainMenu {}

fn despawn_main_menu(mut commands: Commands, query: Query<Entity, With<MainMenu>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn spawn_main_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                background_color: Color::Rgba {
                    red: 0.2,
                    green: 0.2,
                    blue: 0.2,
                    alpha: 0.6,
                }
                .into(),
                ..default()
            },
            MainMenu {},
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "Main Menu",
                TextStyle {
                    font: asset_server.load("fonts/Start.otf"),
                    font_size: 80.0,
                    color: Color::WHITE,
                },
            ));
        });
}

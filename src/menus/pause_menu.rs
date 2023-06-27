use bevy::{app::AppExit, prelude::*};

use crate::{
    config::ConfigRes,
    rl::{ModelPredator, ModelPrey},
    states::AppState,
};

pub struct PauseMenuPlugin;

impl Plugin for PauseMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(spawn_pause_menu.in_schedule(OnEnter(AppState::Paused)))
            .add_system(despawn_pause_menu.in_schedule(OnExit(AppState::Paused)))
            .add_system(button_click_handler.in_set(OnUpdate(AppState::Paused)));
    }
}

#[derive(Component)]
struct PauseMenu {}

#[derive(Component)]
struct ExitButton {}

fn button_click_handler(
    mut exit: EventWriter<AppExit>,
    query: Query<&Interaction, With<ExitButton>>,
    model_prey: Res<ModelPrey>,
    model_predator: Res<ModelPredator>,
    config: Res<ConfigRes>,
) {
    for interaction in &query {
        if *interaction == Interaction::Clicked {
            model_prey.model.save(&config.0.rl.save_path);
            model_predator.model.save(&config.0.rl.save_path);
            exit.send(AppExit);
        }
    }
}

fn despawn_pause_menu(mut commands: Commands, query: Query<Entity, With<PauseMenu>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn spawn_pause_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                    justify_content: JustifyContent::SpaceAround,
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
            PauseMenu {},
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "Paused",
                TextStyle {
                    font: asset_server.load("fonts/Start.otf"),
                    font_size: 80.0,
                    color: Color::WHITE,
                },
            ));
            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            margin: UiRect::top(Val::Px(20.0)),
                            padding: UiRect::all(Val::Px(5.0)),
                            ..default()
                        },
                        background_color: Color::Rgba {
                            red: 0.0,
                            green: 0.0,
                            blue: 0.0,
                            alpha: 0.3,
                        }
                        .into(),
                        ..default()
                    },
                    ExitButton {},
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Save models and exit",
                        TextStyle {
                            font: asset_server.load("fonts/Start.otf"),
                            font_size: 30.0,
                            color: Color::WHITE,
                        },
                    ));
                });
        });
}

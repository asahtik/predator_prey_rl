use bevy::prelude::*;
use bevy_mod_picking::Selection;

use crate::{
    entities::{raycast::Detection, LearnLog},
    states::{AppState, GameState},
};

const BUTTON_SIZE: f32 = 75.0;
const BUTTON_MARGIN: f32 = 10.0;

const COLOR_SELECTED: Color = Color::Rgba {
    red: 1.0,
    green: 1.0,
    blue: 1.0,
    alpha: 0.8,
};
const COLOR_UNSELECTED: Color = Color::Rgba {
    red: 0.0,
    green: 0.0,
    blue: 0.0,
    alpha: 0.3,
};

pub struct GameMenuPlugin;

impl Plugin for GameMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(spawn_game_menu.in_schedule(OnEnter(AppState::InGame)))
            .add_system(despawn_game_menu.in_schedule(OnExit(AppState::InGame)))
            .add_systems(
                (button_click_handler, update_game_menu).in_set(OnUpdate(AppState::InGame)),
            );
    }
}

#[derive(Component)]
struct GameMenu {}

#[derive(Component)]
struct SpeedMenu {}

#[derive(Component)]
struct EnergyData {}

#[derive(Component)]
struct FitnessData {}

#[derive(Component)]
struct StateData {}

#[derive(Component)]
struct GameMenuButton {
    pub next: GameState,
}
impl GameMenuButton {
    pub fn new(next: GameState) -> Self {
        Self { next }
    }
}

fn despawn_game_menu(mut commands: Commands, query: Query<Entity, With<GameMenu>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn button_click_handler(
    mut query: Query<(&Interaction, &GameMenuButton, &mut BackgroundColor)>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, button, mut background_color) in &mut query {
        match interaction {
            Interaction::Clicked => {
                next_state.set(button.next);
                *background_color = COLOR_SELECTED.into();
            }
            _ => {
                if button.next != state.0 {
                    *background_color = COLOR_UNSELECTED.into();
                }
            }
        }
    }
}

fn spawn_menu_top(
    parent: &mut ChildBuilder,
    state: &Res<State<GameState>>,
    asset_server: &Res<AssetServer>,
) {
    parent
        .spawn((
            NodeBundle {
                style: Style {
                    size: Size::new(
                        Val::Px(BUTTON_SIZE * 3.0 + 2.0 * BUTTON_MARGIN),
                        Val::Px(BUTTON_SIZE),
                    ),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Row,
                    ..default()
                },
                ..default()
            },
            SpeedMenu {},
        ))
        .with_children(|parent| {
            parent.spawn((
                ButtonBundle {
                    style: Style {
                        size: Size::new(Val::Px(BUTTON_SIZE), Val::Px(BUTTON_SIZE)),
                        ..default()
                    },
                    image: asset_server.load("icons/play.png").into(),
                    background_color: if state.0 == GameState::Normal {
                        COLOR_SELECTED.into()
                    } else {
                        COLOR_UNSELECTED.into()
                    },
                    ..default()
                },
                GameMenuButton::new(GameState::Normal),
            ));
            parent.spawn((
                ButtonBundle {
                    style: Style {
                        size: Size::new(Val::Px(BUTTON_SIZE), Val::Px(BUTTON_SIZE)),
                        ..default()
                    },
                    image: asset_server.load("icons/ff.png").into(),
                    background_color: if state.0 == GameState::FastForward {
                        COLOR_SELECTED.into()
                    } else {
                        COLOR_UNSELECTED.into()
                    },
                    ..default()
                },
                GameMenuButton::new(GameState::FastForward),
            ));
            parent.spawn((
                ButtonBundle {
                    style: Style {
                        size: Size::new(Val::Px(BUTTON_SIZE), Val::Px(BUTTON_SIZE)),
                        ..default()
                    },
                    image: asset_server.load("icons/skip.png").into(),
                    background_color: if state.0 == GameState::Skip {
                        COLOR_SELECTED.into()
                    } else {
                        COLOR_UNSELECTED.into()
                    },
                    ..default()
                },
                GameMenuButton::new(GameState::Skip),
            ));
        });

    parent.spawn((
        TextBundle::from_section(
            "x",
            TextStyle {
                font: asset_server.load("fonts/Start.otf"),
                font_size: 30.0,
                color: Color::WHITE,
            },
        )
        .with_text_alignment(TextAlignment::Center),
        EnergyData {},
    ));

    parent.spawn((
        TextBundle::from_section(
            "Fitness",
            TextStyle {
                font: asset_server.load("fonts/Start.otf"),
                font_size: 10.0,
                color: Color::WHITE,
            },
        )
        .with_text_alignment(TextAlignment::Center)
        .with_style(Style {
            flex_wrap: FlexWrap::Wrap,
            ..default()
        }),
        FitnessData {},
    ));
}

fn spawn_menu_bottom(parent: &mut ChildBuilder, asset_server: &Res<AssetServer>) {
    parent.spawn((
        TextBundle::from_section(
            "x",
            TextStyle {
                font: asset_server.load("fonts/Start.otf"),
                font_size: 15.0,
                color: Color::WHITE,
            },
        )
        .with_text_alignment(TextAlignment::Center),
        StateData {},
    ));
}

fn spawn_game_menu(
    mut commands: Commands,
    state: Res<State<GameState>>,
    asset_server: Res<AssetServer>,
) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                ..default()
            },
            GameMenu {},
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    NodeBundle {
                        style: Style {
                            size: Size::new(Val::Percent(100.0), Val::Px(BUTTON_SIZE)),
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            flex_direction: FlexDirection::Row,
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
                    GameMenu {},
                ))
                .with_children(|parent| spawn_menu_top(parent, &state, &asset_server));

            parent
                .spawn((
                    NodeBundle {
                        style: Style {
                            size: Size::new(Val::Percent(100.0), Val::Px(BUTTON_SIZE)),
                            justify_content: JustifyContent::SpaceAround,
                            align_items: AlignItems::Center,
                            flex_direction: FlexDirection::Row,
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
                    GameMenu {},
                ))
                .with_children(|parent| spawn_menu_bottom(parent, &asset_server));
        });
}

fn get_state_text(agent: &crate::entities::Agent) -> String {
    if let Some(state) = &agent.state {
        let mut text = " ".to_string();
        for sr in &state.sight {
            let ray_text = match &sr.detection {
                Detection::PreyAlive(_, _) => "Alive",
                Detection::PreyDead(_) => "Dead",
                Detection::Predator(_, _) => "Predator",
                Detection::Wall => "Wall",
                Detection::None => "None",
            };
            text.push_str(&format!("{: ^8} ", ray_text));
        }
        text
    } else {
        "x".to_string()
    }
}

fn update_game_menu(
    query: Query<(Entity, &crate::entities::Agent, &Selection)>,
    learn_data: Res<LearnLog>,
    asset_server: Res<AssetServer>,
    mut text_query: Query<&mut Text, With<EnergyData>>,
    mut fitness_query: Query<&mut Text, (With<FitnessData>, Without<EnergyData>)>,
    mut state_query: Query<&mut Text, (With<StateData>, Without<EnergyData>, Without<FitnessData>)>,
) {
    let sel = query
        .iter()
        .find(|(_, _, sel)| sel.selected())
        .map(|(_, agent, _)| agent);
    if let Some(a) = sel {
        text_query.single_mut().sections[0].value = format!("{:.2}\n{}", a.energy, a.life);
        state_query.single_mut().sections[0].value = get_state_text(a);
        println!("Most recent action: {:?}", a.action);
    } else {
        text_query.single_mut().sections[0].value = "x".to_string();
        state_query.single_mut().sections[0].value = "x".to_string();
    }
    let mut fit_text = fitness_query.single_mut();
    fit_text.sections.clear();
    fit_text.sections.push(TextSection {
        value: format!("Epoch: \n{}\n", learn_data.epoch),
        style: TextStyle {
            font: asset_server.load("fonts/Start.otf"),
            font_size: 10.0,
            color: Color::WHITE,
        },
    });
    fit_text.sections.push(TextSection {
        value: format!("Prey Loss: \n{}\n", learn_data.prey_loss),
        style: TextStyle {
            font: asset_server.load("fonts/Start.otf"),
            font_size: 10.0,
            color: Color::WHITE,
        },
    });
    fit_text.sections.push(TextSection {
        value: format!("Predator Loss: \n{}\n", learn_data.predator_loss),
        style: TextStyle {
            font: asset_server.load("fonts/Start.otf"),
            font_size: 10.0,
            color: Color::WHITE,
        },
    });
}

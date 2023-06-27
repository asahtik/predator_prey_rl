use bevy::{prelude::*, window::PresentMode};

mod assets;
mod camera_control;
mod config;
mod entities;
mod helpers;
mod menus;
mod rl;
mod states;

use bevy_mod_picking::{DebugCursorPickingPlugin, DefaultPickingPlugins, PickingCameraBundle};
use camera_control::{CameraMovementPlugin, PrimaryCamera};
use config::{ConfigRes, Map};
use entities::EntityPlugin;
use menus::MenuPlugins;
use rl::model_helpers::AgentModel;
use states::{AppState, GameState};

#[derive(Component)]
struct Actor {}

fn main() {
    let config = ConfigRes::default();
    let learn = config.0.rl.learn;
    let (prey_model, predator_model) = get_models(&config);
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: if learn {
                    PresentMode::AutoNoVsync
                } else {
                    PresentMode::AutoVsync
                },
                ..default()
            }),
            ..default()
        }))
        // .add_plugin(WorldInspectorPlugin::default())
        .add_plugins(DefaultPickingPlugins)
        .add_plugin(DebugCursorPickingPlugin)
        .insert_resource(rl::ReplayBufferPrey::new(config.0.rl.replay_buffer_size))
        .insert_resource(rl::ReplayBufferPredator::new(
            config.0.rl.replay_buffer_size,
        ))
        .insert_resource(prey_model)
        .insert_resource(predator_model)
        .insert_resource(config)
        .add_asset::<assets::MapAsset>()
        .add_asset_loader(assets::MapLoader)
        .init_resource::<Map>()
        .add_state::<AppState>()
        .add_state::<GameState>()
        .add_plugins(MenuPlugins)
        .add_plugin(CameraMovementPlugin)
        .add_plugin(EntityPlugin)
        .add_startup_system(setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut map: ResMut<Map>,
    mut game_state: ResMut<NextState<GameState>>,
    mut app_state: ResMut<NextState<AppState>>,
    config: Res<ConfigRes>,
    assets: Res<AssetServer>,
) {
    map.map = assets.load(config.0.world.map.clone() + ".map");

    if config.0.rl.learn {
        game_state.set(GameState::Skip);
        app_state.set(AppState::InGame)
    }

    // plane
    let world_size = Vec2::new(config.0.world.world_width, config.0.world.world_height);
    let map_path = config.0.world.map.clone();
    commands.spawn(PbrBundle {
        mesh: meshes.add(
            shape::Quad {
                size: world_size,
                flip: false,
            }
            .into(),
        ),
        material: materials.add(assets.load(map_path + ".bmp").into()),
        transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ..default()
    });
    // light
    commands.insert_resource(AmbientLight {
        brightness: 0.2,
        ..default()
    });
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            illuminance: 4000.0,
            ..default()
        },
        transform: Transform::from_xyz(
            world_size.x / 4.0,
            world_size.max_element() / 2.0,
            world_size.y / 4.0,
        )
        .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    // camera
    commands
        .spawn((
            Camera3dBundle {
                transform: Transform::from_xyz(
                    0.0,
                    world_size.max_element() / 4.0,
                    world_size.y / 2.0,
                )
                .looking_at(Vec3::ZERO, Vec3::Y),
                ..default()
            },
            PrimaryCamera {
                radius: config.0.camera.default_radius,
            },
        ))
        .insert(PickingCameraBundle::default());
}

fn get_models(cfg: &ConfigRes) -> (rl::ModelPrey, rl::ModelPredator) {
    if !std::path::Path::new(&cfg.0.rl.save_path).is_dir() {
        std::fs::create_dir_all(&cfg.0.rl.save_path)
            .expect("Save path does not exist, could not create");
    }
    if let Some(path) = &cfg.0.rl.load_path {
        assert!(std::path::Path::new(&path).is_dir());
        if let Some(name) = &cfg.0.rl.load_model_name {
            (
                rl::ModelPrey {
                    model: AgentModel::load(path, format!("prey_{name}").as_str(), cfg.0.rl.learning_rate),
                },
                rl::ModelPredator {
                    model: AgentModel::load(path, format!("predator_{name}").as_str(), cfg.0.rl.learning_rate),
                },
            )
        } else {
            let mut files = std::fs::read_dir(path)
                .unwrap()
                .filter_map(|f| {
                    f.ok().and_then(|f| {
                        f.file_name().into_string().ok().and_then(|f| {
                            std::path::Path::new(&f).file_stem().and_then(|f| {
                                f.to_str().and_then(|f| {
                                    f.split_once('_')
                                        .map(|(t, n)| (t.to_string(), n.to_string()))
                                })
                            })
                        })
                    })
                })
                .collect::<Vec<_>>();
            files.sort_by(|a, b| b.1.cmp(&a.1));
            let prey_name = files
                .iter()
                .find(|(t, _)| *t == "prey")
                .map(|f| f.1.clone())
                .expect("Could not find an appropriate prey model");
            let predator_name = files
                .iter()
                .find(|(t, _)| *t == "predator")
                .map(|f| f.1.clone())
                .expect("Could not find an appropriate predator model");
            println!("Prey model: {prey_name}");
            println!("Predator model: {predator_name}");
            (
                rl::ModelPrey {
                    model: AgentModel::load(path, format!("prey_{prey_name}").as_str(), cfg.0.rl.learning_rate),
                },
                rl::ModelPredator {
                    model: AgentModel::load(path, format!("predator_{predator_name}").as_str(), cfg.0.rl.learning_rate),
                },
            )
        }
    } else {
        (
            rl::ModelPrey {
                model: AgentModel::new(
                    11 + cfg.0.prey.vision_rays * 15 + cfg.0.prey.hearing_rays * 10,
                    11,
                    &cfg.0.rl.layers,
                    cfg.0.rl.learning_rate,
                    entities::AgentType::Prey,
                ),
            },
            rl::ModelPredator {
                model: AgentModel::new(
                    11 + cfg.0.predator.vision_rays * 15 + cfg.0.predator.hearing_rays * 10,
                    11,
                    &cfg.0.rl.layers,
                    cfg.0.rl.learning_rate,
                    entities::AgentType::Predator,
                ),
            },
        )
    }
}

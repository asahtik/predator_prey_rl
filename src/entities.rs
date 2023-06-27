use std::f32::consts::FRAC_PI_2;
use std::num::Wrapping;

use bevy::app::AppExit;
use bevy::prelude::*;
use rayon::prelude::*;

use crate::assets::MapAsset;
use crate::config::{self, ConfigRes, Map, INITIAL_ENERGY, MAX_ENERGY};
use crate::helpers::map;
use crate::helpers::map::EnvType;
use crate::rl::model::NormalizationData;
use crate::rl::{self, ModelPredator, ModelPrey, Transition};

use crate::states::{AppState, GameState};

use self::go::control_agent;
use self::preprocessing::{preprocess_predator, preprocess_prey};
use self::raycast::{cast_rays_hearing, cast_rays_vision, RayDetection};
use self::spawning::batch_spawn;

mod bbox;
mod go;
mod intersect;
mod preprocessing;
pub mod raycast;
mod spawning;

pub struct ResetEvent;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
enum ExecSet {
    Prepare,   // Setup timers
    Calculate, // Spawn (if procreated), reduce & despawn (if dead and eaten), set state, energy update, rewards and replay buffer update, Calculation
    Update,    // Update model (only run every n frames)
    Render,
}

#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
pub enum AgentType {
    Prey,
    Predator,
}

#[derive(Debug, Clone, Copy, PartialEq, Reflect, FromReflect)]
pub enum TurnDirection {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
pub enum Action {
    Turn(TurnDirection),
    Walk,
    TurnWalk(TurnDirection),
    Run,
    TurnRun(TurnDirection),
    Procreate(Option<Entity>),
    Eat(Option<Entity>),
    None,
}
impl Action {
    pub fn from_action_index(idx: usize) -> Self {
        match idx {
            0 => Action::Turn(TurnDirection::Left),
            1 => Action::Turn(TurnDirection::Right),
            2 => Action::Walk,
            3 => Action::TurnWalk(TurnDirection::Left),
            4 => Action::TurnWalk(TurnDirection::Right),
            5 => Action::Run,
            6 => Action::TurnRun(TurnDirection::Left),
            7 => Action::TurnRun(TurnDirection::Right),
            8 => Action::Procreate(None),
            9 => Action::Eat(None),
            10 => Action::None,
            _ => panic!("Invalid action index"),
        }
    }
    pub fn to_action_index(self) -> usize {
        match self {
            Action::Turn(TurnDirection::Left) => 0,
            Action::Turn(TurnDirection::Right) => 1,
            Action::Walk => 2,
            Action::TurnWalk(TurnDirection::Left) => 3,
            Action::TurnWalk(TurnDirection::Right) => 4,
            Action::Run => 5,
            Action::TurnRun(TurnDirection::Left) => 6,
            Action::TurnRun(TurnDirection::Right) => 7,
            Action::Procreate(_) => 8,
            Action::Eat(_) => 9,
            Action::None => 10,
        }
    }
}

#[derive(Clone, Debug, Reflect, FromReflect)]
pub struct AgentState {
    pub location: Vec2,
    pub direction: f32,
    pub speed: f32,
    pub energy: f32,
    pub environment: EnvType,
    pub sight: Vec<RayDetection>,
    pub hearing: Vec<RayDetection>,
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct Agent {
    pub agent_type: AgentType,
    pub speed: f32,
    pub location: Vec2,
    pub direction: f32,
    pub action: Action,
    pub energy: f32,
    pub alive: bool,
    pub life: usize,
    pub state: Option<AgentState>,
    pub previous_state: Option<AgentState>,
}
impl Agent {
    pub fn new(t: AgentType, loc: Vec2, dir: f32, life: usize) -> Self {
        Self {
            agent_type: t,
            speed: 0.0,
            location: loc,
            direction: dir,
            action: Action::None,
            energy: INITIAL_ENERGY,
            alive: true,
            life,
            state: None,
            previous_state: None,
        }
    }
    pub fn set_state(&mut self, new_state: AgentState) {
        std::mem::swap(&mut self.state, &mut self.previous_state);
        self.state = Some(new_state);
    }
}

#[derive(Resource, Default, Debug)]
struct LearnLogT {
    pub epoch: usize,
    pub prey_loss: f32,
    pub predator_loss: f32,
}

#[derive(Resource, Default, Debug)]
pub struct LearnLog {
    pub epoch: usize,
    pub prey_loss: f32,
    pub predator_loss: f32,
}

#[derive(Resource, Default, Debug)]
struct UpdateTimer {
    counter1: Wrapping<usize>,
    counter2: Wrapping<usize>,
}

#[derive(Resource, Default, Debug)]
struct ResetTimer {
    counter: Wrapping<usize>,
}

#[derive(Resource)]
struct FrameTimer {
    timer: Timer,
}
impl Default for FrameTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(1.0 / config::FPS, TimerMode::Repeating),
        }
    }
}

fn should_run_frame(game_state: Res<State<GameState>>, timer: Res<FrameTimer>) -> bool {
    if game_state.0 == GameState::Normal {
        timer.timer.finished()
    } else {
        true
    }
}

pub struct EntityPlugin;
impl Plugin for EntityPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            (
                ExecSet::Prepare,
                ExecSet::Calculate.run_if(should_run_frame),
                ExecSet::Render,
                ExecSet::Update.run_if(should_run_frame),
            )
                .chain()
                .in_set(OnUpdate(AppState::InGame)),
        )
        .register_type::<Agent>()
        .add_event::<ResetEvent>()
        .init_resource::<LearnLogT>()
        .init_resource::<LearnLog>()
        .init_resource::<FrameTimer>()
        .init_resource::<UpdateTimer>()
        .init_resource::<ResetTimer>()
        .add_startup_system(spawn_agents)
        .add_system(update_frame_timer.in_set(ExecSet::Prepare))
        .add_system(update_learn_log.in_set(ExecSet::Prepare))
        .add_systems(
            (preprocess_agents, move_agents)
                .chain()
                .in_set(ExecSet::Calculate),
        )
        .add_system(
            render_agents
                .in_set(ExecSet::Render)
                .run_if(in_state(GameState::Normal).or_else(in_state(GameState::FastForward))),
        )
        .add_systems(
            (update_models, reset_environment)
                .chain()
                .in_set(ExecSet::Update),
        );
    }
}

fn update_frame_timer(mut timer: ResMut<FrameTimer>, time: Res<Time>) {
    timer.timer.tick(time.delta());
}

fn update_learn_log(mut log: ResMut<LearnLog>, logt: Res<LearnLogT>) {
    log.epoch = logt.epoch;
    log.prey_loss = logt.prey_loss;
    log.predator_loss = logt.predator_loss;
}

fn preprocess_agents(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Agent)>,
    mut prey_buf: ResMut<rl::ReplayBufferPrey>,
    mut predator_buf: ResMut<rl::ReplayBufferPredator>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut res_ev: EventWriter<ResetEvent>,
    assets: Res<AssetServer>,
    config: Res<ConfigRes>,
    map_res: Res<Map>,
    map: Res<Assets<MapAsset>>,
) {
    for (e, mut a) in &mut query {
        if a.alive {
            a.life -= 1;
        }
        if a.life == 0 {
            a.alive = false;
            commands.entity(e).despawn_recursive();
        }
    }
    if query
        .iter()
        .filter(|(_, a)| a.agent_type == AgentType::Predator && a.alive)
        .count()
        == 0
        || query
            .iter()
            .filter(|(_, a)| a.agent_type == AgentType::Prey && a.alive)
            .count()
            == 0
    {
        res_ev.send(ResetEvent);
    } else {
        let map = map.get(&map_res.map).unwrap();
        let world_size = Vec2::new(config.0.world.world_width, config.0.world.world_height);
        let half_size = world_size / 2.0;

        let world_borders = (-half_size, half_size);

        let mut corpses = query
            .iter()
            .filter(|(_, a)| a.agent_type == AgentType::Prey && !a.alive)
            .map(|(e, a)| (e, a.location, a.direction, a.energy))
            .collect::<Vec<_>>();
        let mut preys = query
            .iter()
            .filter(|(_, a)| a.agent_type == AgentType::Prey && a.alive)
            .map(|(e, a)| (e, a.location, a.direction, a.energy, 0u32))
            .collect::<Vec<_>>();
        let predators = query
            .iter()
            .filter(|(_, a)| a.agent_type == AgentType::Predator && a.alive)
            .map(|(e, a)| (e, a.location, a.direction, a.energy))
            .collect::<Vec<_>>();

        // Predators
        preprocess_predator(
            &mut commands,
            &mut query,
            &mut predator_buf,
            &mut meshes,
            &mut materials,
            &assets,
            &config,
            map,
            &mut corpses,
            &mut preys,
            &predators,
            world_borders,
            half_size,
        );

        // Despawn consumed corpses
        for (e, .., en) in &corpses {
            if *en <= 0.0 {
                commands.entity(*e).despawn_recursive();
            }
        }

        // Corpses
        for ((_, mut a), (_, _, _, en)) in &mut query
            .iter_mut()
            .filter(|(_, a)| a.agent_type == AgentType::Prey && !a.alive)
            .zip(&corpses)
        {
            a.energy = *en;
        }

        // Preys
        preprocess_prey(
            &mut commands,
            &mut query,
            &mut prey_buf,
            &mut meshes,
            &mut materials,
            &assets,
            &config,
            map,
            &corpses,
            &preys,
            &predators,
            world_borders,
            half_size,
        );
    }
}

pub fn move_agents(
    mut query: Query<(Entity, &mut Agent)>,
    config: Res<ConfigRes>,
    model_prey: Res<ModelPrey>,
    model_predator: Res<ModelPredator>,
    map_res: Res<Map>,
    map: Res<Assets<MapAsset>>,
) {
    let world_size = Vec2::new(config.0.world.world_width, config.0.world.world_height);
    let half_size = world_size / 2.0;
    let world_borders = (-half_size, half_size);
    let map = map.get(&map_res.map).unwrap();

    let preys = query
        .iter()
        .filter(|(_, a)| a.agent_type == AgentType::Prey)
        .map(|(e, a)| (e, a))
        .collect::<Vec<_>>();
    let predators = query
        .iter()
        .filter(|(_, a)| a.agent_type == AgentType::Predator)
        .map(|(e, a)| (e, a))
        .collect::<Vec<_>>();
    let agents = query.iter().collect::<Vec<_>>();
    let new_agents = agents
        .par_iter()
        .map(|(e, a)| {
            let action = if a.agent_type == AgentType::Prey {
                model_prey.model.get_action(
                    a.state.as_ref().unwrap(),
                    &config.0.prey,
                    world_borders,
                    config.0.rl.learn,
                )
            } else {
                model_predator.model.get_action(
                    a.state.as_ref().unwrap(),
                    &config.0.predator,
                    world_borders,
                    config.0.rl.learn,
                )
            };
            // println!("Chosen action: {action:?}");
            (
                *e,
                control_agent((*e, a), &config, &preys, &predators, action, map),
            )
        })
        .collect::<Vec<_>>();
    for ((_, mut a), (_, new_agent)) in query.iter_mut().zip(new_agents) {
        *a = new_agent;
    }
}

fn render_agents(
    mut query: Query<(&Agent, &mut Transform), Changed<Agent>>,
    config: Res<ConfigRes>,
) {
    for (agent, mut transform) in &mut query {
        transform.translation = Vec3::new(agent.location.x, 0.0, agent.location.y);
        transform.rotation = Quat::from_rotation_y(agent.direction);
        if !agent.alive {
            let cfg = match agent.agent_type {
                AgentType::Prey => &config.0.prey,
                AgentType::Predator => &config.0.predator,
            };
            let dir_vec = Vec3::new(agent.direction.cos(), 0.0, -agent.direction.sin());
            transform.rotate(Quat::from_axis_angle(dir_vec, FRAC_PI_2));
            let perp_dir_vec = Vec3::new(dir_vec.z, 0.0, -dir_vec.x);
            transform.translation += perp_dir_vec * 0.5 * cfg.size * cfg.hl_ratio;
        }
    }
}

fn update_models(
    mut model_prey: ResMut<ModelPrey>,
    mut model_predator: ResMut<ModelPredator>,
    mut update_timer: ResMut<UpdateTimer>,
    mut exit: EventWriter<AppExit>,
    mut logt: ResMut<LearnLogT>,
    prey_buf: Res<rl::ReplayBufferPrey>,
    predator_buf: Res<rl::ReplayBufferPredator>,
    config: Res<ConfigRes>,
) {
    update_timer.counter1 += 1;
    let cfg = &config.0.rl;
    if update_timer.counter1.0 % cfg.frames_per_update == 0 {
        update_timer.counter1.0 = 0;
        update_timer.counter2 += 1;
        logt.epoch = update_timer.counter2.0;
        if update_timer.counter2.0 % cfg.updates_per_save == 0 {
            model_prey.model.save(&cfg.save_path);
            model_predator.model.save(&cfg.save_path);
        }
        if update_timer.counter2.0 % cfg.updates_per_target == 0 {
            model_prey.model.reset_target();
            model_predator.model.reset_target();
        }
        if !cfg.learn {
            return;
        }
        let mut rng = rand::thread_rng();
        let world_size = Vec2::new(config.0.world.world_width, config.0.world.world_height);
        let half_size = world_size / 2.0;
        let world_borders = (-half_size, half_size);

        let predator_buf = predator_buf.buffer.get();
        // println!("Predator: {:?}", predator_buf.iter().map(|a| a.action).collect::<Vec<_>>());
        let prey_buf = prey_buf.buffer.get();
        // println!("Prey: {:?}", prey_buf.iter().map(|a| a.action).collect::<Vec<_>>());
        let swap = if let Some(s) = cfg.updates_per_swap {
            Some((update_timer.counter2.0 / s) % 2)
        } else {
            None
        };

        if swap.is_none() || swap.unwrap() == 0 {
            logt.predator_loss = model_predator.model.backpropagate(
                &predator_buf,
                &NormalizationData {
                    min_speed: 0.0,
                    max_speed: config.0.predator.run_speed,
                    min_loc: world_borders.0,
                    max_loc: world_borders.1,
                    min_energy: 0.0,
                    max_energy: 100.0,
                    min_dist: 0.0,
                    max_dist: config.0.predator.vision_range,
                },
                &config.0.rl,
                &mut rng,
            );
        }
        if swap.is_none() || swap.unwrap() == 1 {
            logt.prey_loss = model_prey.model.backpropagate(
                &prey_buf,
                &NormalizationData {
                    min_speed: 0.0,
                    max_speed: config.0.prey.run_speed,
                    min_loc: world_borders.0,
                    max_loc: world_borders.1,
                    min_energy: 0.0,
                    max_energy: 100.0,
                    min_dist: 0.0,
                    max_dist: config.0.prey.vision_range,
                },
                &config.0.rl,
                &mut rng,
            );
        }

        if update_timer.counter2.0 >= cfg.num_updates {
            if update_timer.counter2.0 % cfg.updates_per_save != 0 {
                model_prey.model.save(&cfg.save_path);
                model_predator.model.save(&cfg.save_path);
            }
            exit.send(AppExit);
        }
    }
}

fn respawn(
    mut commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    query: Query<Entity, With<Agent>>,
    config: Res<ConfigRes>,
    assets: Res<AssetServer>,
) {
    println!("Resetting environment");
    for e in query.iter() {
        commands.entity(e).despawn_recursive();
    }
    spawn_agents(commands, meshes, materials, config, assets);
}

fn reset_environment(
    commands: Commands,
    mut reset_timer: ResMut<ResetTimer>,
    mut res_ev: EventReader<ResetEvent>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    query: Query<Entity, With<Agent>>,
    config: Res<ConfigRes>,
    assets: Res<AssetServer>,
    update_timer: Res<UpdateTimer>,
) {
    let cfg = &config.0.rl;
    let mut cnt = 0u32;
    for _ in res_ev.iter() {
        cnt += 1;
    }
    if cnt >= 1 {
        reset_timer.counter.0 = 0;
        respawn(commands, meshes, materials, query, config, assets);
    } else if update_timer.counter1.0 % cfg.frames_per_update == 0 {
        reset_timer.counter += 1;
        if reset_timer.counter.0 % cfg.updates_per_reset == 0 {
            reset_timer.counter.0 = 0;
            respawn(commands, meshes, materials, query, config, assets);
        }
    }
}

fn spawn_agents(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<ConfigRes>,
    assets: Res<AssetServer>,
) {
    let world_size = Vec2::new(config.0.world.world_width, config.0.world.world_height);
    let prey_mesh = assets.load("models/deer.glb#Scene0");
    let predator_mesh = assets.load("models/wolf.glb#Scene0");
    let prey_scale = config.0.prey.size;
    let predator_scale = config.0.predator.size;
    let prey_bb = shape::Box {
        min_x: -prey_scale / 2.0,
        max_x: prey_scale / 2.0,
        min_y: 0.0,
        max_y: prey_scale * config.0.prey.hl_ratio,
        min_z: -prey_scale * config.0.prey.wl_ratio / 2.0,
        max_z: prey_scale * config.0.prey.wl_ratio / 2.0,
    };
    let predator_bb = shape::Box {
        min_x: -predator_scale / 2.0,
        max_x: predator_scale / 2.0,
        min_y: 0.0,
        max_y: predator_scale * config.0.predator.hl_ratio,
        min_z: -predator_scale * config.0.predator.wl_ratio / 2.0,
        max_z: predator_scale * config.0.predator.wl_ratio / 2.0,
    };

    batch_spawn(
        &mut commands,
        &mut meshes,
        &mut materials,
        world_size,
        &prey_mesh,
        prey_scale,
        prey_bb,
        AgentType::Prey,
        config.0.prey.count,
        config.0.world.batch_spawn_count,
        config.0.world.batch_spawn_radius,
        config.0.prey.life,
    );
    batch_spawn(
        &mut commands,
        &mut meshes,
        &mut materials,
        world_size,
        &predator_mesh,
        predator_scale,
        predator_bb,
        AgentType::Predator,
        config.0.predator.count,
        config.0.world.batch_spawn_count,
        config.0.world.batch_spawn_radius,
        config.0.prey.life,
    );
}

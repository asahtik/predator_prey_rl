use crate::{assets::MapAsset, helpers::config_parser::AgentConfig};

use super::{raycast::Detection, spawning::spawn, *};

pub fn preprocess_predator(
    commands: &mut Commands,
    query: &mut Query<(Entity, &mut Agent)>,
    buf: &mut ResMut<rl::ReplayBufferPredator>,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    assets: &Res<AssetServer>,
    config: &Res<ConfigRes>,
    map: &MapAsset,
    corpses: &mut [(Entity, Vec2, f32, f32)],
    preys: &mut [(Entity, Vec2, f32, f32, u32)],
    predators: &[(Entity, Vec2, f32, f32)],
    world_borders: (Vec2, Vec2),
    half_size: Vec2,
) {
    let predator_mesh = assets.load("models/wolf.glb#Scene0");
    let predator_scale = config.0.predator.size;
    let predator_bb = shape::Box {
        min_x: -predator_scale / 2.0,
        max_x: predator_scale / 2.0,
        min_y: 0.0,
        max_y: predator_scale * config.0.predator.hl_ratio,
        min_z: -predator_scale * config.0.predator.wl_ratio / 2.0,
        max_z: predator_scale * config.0.predator.wl_ratio / 2.0,
    };
    let mut predator_procreations =
        std::collections::HashMap::<Entity, bool>::with_capacity(predators.len());
    query
        .iter()
        .filter(|(_, a)| {
            if a.agent_type == AgentType::Predator
                && a.alive
                && a.energy >= config.0.prey.procreation_min_energy
            {
                matches!(a.action, Action::Procreate(Some(_)))
            } else {
                false
            }
        })
        .for_each(|(e, a)| {
            if let Action::Procreate(Some(e2)) = a.action {
                let reciprocated = predator_procreations.contains_key(&e);
                predator_procreations.insert(e2, reciprocated);
                if let Some(rec) = predator_procreations.get_mut(&e) {
                    *rec = true;
                }
            }
        });

    for (e, mut a) in query
        .iter_mut()
        .filter(|(_, a)| a.agent_type == AgentType::Predator && a.alive)
    {
        let cfg = &config.0.predator;
        a.energy -= cfg.tick_energy_loss;

        // Actions
        let mut has_eaten = false;
        let mut has_procreated = false;
        if let Action::Eat(Some(e)) = a.action {
            let corpse = corpses.iter_mut().find(|(e2, _, _, _)| *e2 == e);
            if let Some((_, _, _, energy)) = corpse {
                if *energy >= cfg.eating_speed && a.energy < MAX_ENERGY {
                    *energy -= cfg.eating_speed;
                    a.energy += cfg.eating_speed;
                    if a.energy > MAX_ENERGY {
                        a.energy = MAX_ENERGY;
                    }
                    has_eaten = true;
                } else {
                    *energy = 0.0;
                }
            } else {
                let prey = preys.iter_mut().find(|(e2, _, _, _, _)| *e2 == e);
                if let Some((.., killed)) = prey {
                    *killed += 1;
                    if a.energy < MAX_ENERGY {
                        a.energy += cfg.eating_speed;
                        has_eaten = true;
                    }
                }
                a.energy -= cfg.attack_energy_loss;
            }
        } else if let Action::Procreate(Some(e_partner)) = a.action {
            if a.energy >= cfg.procreation_min_energy {
                let partner = predator_procreations.get(&e);
                if let Some(rec) = partner {
                    if *rec {
                        if e < e_partner {
                            spawn(
                                commands,
                                meshes,
                                materials,
                                &predator_mesh,
                                predator_scale,
                                predator_bb,
                                AgentType::Predator,
                                a.location,
                                a.direction,
                                cfg.life,
                            );
                        }
                        has_procreated = true;
                    }
                }
            }
            a.energy -= if has_procreated {
                cfg.procreation_energy_loss
            } else {
                cfg.procreation_attempt_energy_loss
            };
        } else if matches!(
            a.action,
            Action::Run | Action::TurnRun(TurnDirection::Left | TurnDirection::Right)
        ) {
            a.energy -= cfg.run_energy_loss;
        } else if matches!(
            a.action,
            Action::Walk | Action::TurnWalk(TurnDirection::Left | TurnDirection::Right)
        ) {
            a.energy -= cfg.walk_energy_loss;
        } else if matches!(
            a.action,
            Action::Turn(TurnDirection::Left | TurnDirection::Right)
        ) {
            a.energy -= cfg.turn_energy_loss;
        }

        let new_state = AgentState {
            location: a.location,
            direction: a.direction,
            speed: a.speed,
            energy: a.energy,
            environment: map.0.get_env_type(
                a.location,
                (-half_size.x, half_size.x),
                (-half_size.y, half_size.y),
            ),
            sight: cast_rays_vision(
                e,
                a.agent_type,
                a.location,
                a.direction,
                world_borders,
                map,
                cfg.vision_fov,
                cfg.vision_range,
                cfg.vision_rays,
                corpses,
                preys,
                predators,
                &config.0,
            ),
            hearing: cast_rays_hearing(
                e,
                a.agent_type,
                a.location,
                a.direction,
                world_borders,
                map,
                cfg.hearing_range,
                cfg.hearing_rays,
                corpses,
                preys,
                predators,
                &config.0,
            ),
        };
        a.set_state(new_state);

        // Rewards
        let reward = calculate_rewards(
            a.action,
            a.state.as_ref().unwrap(),
            a.energy <= 0.0,
            has_eaten,
            has_procreated,
            false,
            cfg,
        );
        if a.energy <= 0.0 {
            commands.entity(e).despawn_recursive();
        }

        if let Some(previous_state) = &a.previous_state {
            buf.buffer.add(Transition {
                state: previous_state.clone(),
                action: a.action,
                reward,
                next_state: a.state.as_ref().unwrap().clone(),
            });
        }
    }
}

pub fn preprocess_prey(
    commands: &mut Commands,
    query: &mut Query<(Entity, &mut Agent)>,
    buf: &mut ResMut<rl::ReplayBufferPrey>,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    assets: &Res<AssetServer>,
    config: &Res<ConfigRes>,
    map: &MapAsset,
    corpses: &[(Entity, Vec2, f32, f32)],
    preys: &[(Entity, Vec2, f32, f32, u32)],
    predators: &[(Entity, Vec2, f32, f32)],
    world_borders: (Vec2, Vec2),
    half_size: Vec2,
) {
    let prey_mesh = assets.load("models/deer.glb#Scene0");
    let prey_scale = config.0.prey.size;
    let prey_bb = shape::Box {
        min_x: -prey_scale / 2.0,
        max_x: prey_scale / 2.0,
        min_y: 0.0,
        max_y: prey_scale * config.0.prey.hl_ratio,
        min_z: -prey_scale * config.0.prey.wl_ratio / 2.0,
        max_z: prey_scale * config.0.prey.wl_ratio / 2.0,
    };
    let mut prey_procreations =
        std::collections::HashMap::<Entity, bool>::with_capacity(preys.len());
    query
        .iter()
        .filter(|(_, a)| {
            if a.agent_type == AgentType::Prey
                && a.alive
                && a.energy >= config.0.prey.procreation_min_energy
            {
                matches!(a.action, Action::Procreate(Some(_)))
            } else {
                false
            }
        })
        .for_each(|(e, a)| {
            if let Action::Procreate(Some(e2)) = a.action {
                let reciprocated = prey_procreations.contains_key(&e);
                prey_procreations.insert(e2, reciprocated);
                if let Some(rec) = prey_procreations.get_mut(&e) {
                    *rec = true;
                }
            }
        });

    for ((e, mut a), (_, _, _, _, killed)) in query
        .iter_mut()
        .filter(|(_, a)| a.agent_type == AgentType::Prey && a.alive)
        .zip(preys)
    {
        let cfg = &config.0.prey;
        let mut has_eaten = false;
        let mut has_procreated = false;
        if *killed > 0 {
            a.energy = config.0.predator.eating_speed * config.0.predator.food_quantity as f32;
            a.energy -= config.0.predator.eating_speed * (*killed as f32);
            a.alive = false;
        } else {
            a.energy -= cfg.tick_energy_loss;

            // Actions
            if let Action::Eat(_) = a.action {
                let current_env = map.0.get_env_type(
                    a.location,
                    (world_borders.0.x, world_borders.1.x),
                    (world_borders.0.y, world_borders.1.y),
                );
                if current_env == EnvType::Food && a.energy < MAX_ENERGY {
                    a.energy += cfg.eating_speed;
                    if a.energy > MAX_ENERGY {
                        a.energy = MAX_ENERGY;
                    }
                    has_eaten = true;
                }
            } else if let Action::Procreate(Some(e_partner)) = a.action {
                if a.energy >= cfg.procreation_min_energy {
                    let partner = prey_procreations.get(&e);
                    if let Some(rec) = partner {
                        if *rec {
                            if e < e_partner {
                                spawn(
                                    commands,
                                    meshes,
                                    materials,
                                    &prey_mesh,
                                    prey_scale,
                                    prey_bb,
                                    AgentType::Prey,
                                    a.location,
                                    a.direction,
                                    cfg.life,
                                );
                            }
                            has_procreated = true;
                        }
                    }
                }
                a.energy -= if has_procreated {
                    cfg.procreation_energy_loss
                } else {
                    cfg.procreation_attempt_energy_loss
                };
            } else if matches!(
                a.action,
                Action::Run | Action::TurnRun(TurnDirection::Left | TurnDirection::Right)
            ) {
                a.energy -= cfg.run_energy_loss;
            } else if matches!(
                a.action,
                Action::Walk | Action::TurnWalk(TurnDirection::Left | TurnDirection::Right)
            ) {
                a.energy -= cfg.walk_energy_loss;
            } else if matches!(
                a.action,
                Action::Turn(TurnDirection::Left | TurnDirection::Right)
            ) {
                a.energy -= cfg.turn_energy_loss;
            }

            let new_state = AgentState {
                location: a.location,
                direction: a.direction,
                speed: a.speed,
                energy: a.energy,
                environment: map.0.get_env_type(
                    a.location,
                    (-half_size.x, half_size.x),
                    (-half_size.y, half_size.y),
                ),
                sight: cast_rays_vision(
                    e,
                    a.agent_type,
                    a.location,
                    a.direction,
                    world_borders,
                    map,
                    cfg.vision_fov,
                    cfg.vision_range,
                    cfg.vision_rays,
                    corpses,
                    preys,
                    predators,
                    &config.0,
                ),
                hearing: cast_rays_hearing(
                    e,
                    a.agent_type,
                    a.location,
                    a.direction,
                    world_borders,
                    map,
                    cfg.hearing_range,
                    cfg.hearing_rays,
                    corpses,
                    preys,
                    predators,
                    &config.0,
                ),
            };
            a.set_state(new_state);
        }

        // Rewards
        let reward = calculate_rewards(
            a.action,
            a.state.as_ref().unwrap(),
            a.energy <= 0.0,
            has_eaten,
            has_procreated,
            *killed > 0,
            cfg,
        );
        if *killed == 0 && a.energy <= 0.0 {
            commands.entity(e).despawn_recursive();
        }

        if let Some(previous_state) = a.previous_state.as_ref() {
            buf.buffer.add(Transition {
                state: previous_state.clone(),
                action: a.action,
                reward,
                next_state: a.state.as_ref().unwrap().clone(),
            });
        }
    }
}

fn calculate_rewards(
    action: Action,
    new_state: &AgentState,
    died: bool,
    has_eaten: bool,
    has_procreated: bool,
    killed: bool,
    config: &AgentConfig,
) -> f32 {
    let mut reward = config.rewards.tick;
    if died || killed {
        reward += config.rewards.death;
    }
    match action {
        Action::Procreate(_) => {
            if has_procreated {
                reward += config.rewards.procreation;
            }
        }
        Action::Eat(_) => {
            if has_eaten {
                reward += config.rewards.eat;
            }
        }
        Action::Turn(_) => reward += config.rewards.turn,
        Action::Walk | Action::TurnWalk(_) => reward += config.rewards.walk,
        Action::Run | Action::TurnRun(_) => reward += config.rewards.run,
        Action::None => {}
    }
    let mut num_preys = 0;
    let mut num_predators = 0;
    let mut num_food = 0;
    for d in new_state.sight.iter().chain(new_state.hearing.iter()) {
        match d.detection {
            Detection::PreyAlive(..) => {
                num_preys += 1;
            }
            Detection::Predator(..) => {
                num_predators += 1;
            }
            _ => {}
        }
        if d.food {
            num_food += 1;
        }
    }
    reward += config.rewards.detecting_food * num_food as f32
        + config.rewards.detecting_prey * num_preys as f32
        + config.rewards.detecting_predator * num_predators as f32;
    
    reward
}

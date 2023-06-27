use std::f32::consts::PI;

use bevy::prelude::*;
use rand::seq::IteratorRandom;

use crate::{assets::MapAsset, config::ConfigRes, helpers::map};

use super::{bbox::get_intersecting_agents, Action, Agent, AgentType, TurnDirection};

fn eat_predator(
    agent: &mut Agent,
    selected: &(Entity, &Agent),
    preys: &[(Entity, &Agent)],
    bbox_shape_self: Vec2,
    bbox_shape_prey_alive: Vec2,
    bbox_shape_prey_dead: Vec2,
) {
    let mut rng = rand::thread_rng();

    let alive_targets = preys
        .iter()
        .filter(|a| a.1.alive)
        .copied()
        .collect::<Vec<_>>();
    let alive_targets = get_intersecting_agents(
        selected,
        &alive_targets,
        bbox_shape_self,
        bbox_shape_prey_alive,
    );
    if let Some(target) = alive_targets.iter().choose(&mut rng) {
        agent.action = Action::Eat(Some(target.0));
    } else {
        let dead_targets = preys
            .iter()
            .filter(|a| !a.1.alive)
            .copied()
            .collect::<Vec<_>>();
        let dead_targets = get_intersecting_agents(
            selected,
            &dead_targets,
            bbox_shape_self,
            bbox_shape_prey_dead,
        );
        if let Some(target) = dead_targets.iter().choose(&mut rng) {
            agent.action = Action::Eat(Some(target.0));
        } else {
            agent.action = Action::Eat(None);
        }
    }
}
fn eat_prey(agent: &mut Agent) {
    agent.action = Action::Eat(None);
}

pub fn control_agent(
    selected: (Entity, &Agent),
    config: &ConfigRes,
    preys: &[(Entity, &Agent)],
    predators: &[(Entity, &Agent)],
    action: Action,
    map: &MapAsset,
) -> Agent {
    let mut rng = rand::thread_rng();

    let mut agent = selected.1.clone();
    if !agent.alive {
        return agent;
    }

    let world_size = Vec2::new(config.0.world.world_width, config.0.world.world_height);
    let half_size = world_size / 2.0;

    let walk_acceleration = match agent.agent_type {
        AgentType::Prey => config.0.prey.walk_acceleration,
        AgentType::Predator => config.0.predator.walk_acceleration,
    };
    let run_acceleration = match agent.agent_type {
        AgentType::Prey => config.0.prey.run_acceleration,
        AgentType::Predator => config.0.predator.run_acceleration,
    };
    let walk_top_speed = match agent.agent_type {
        AgentType::Prey => config.0.prey.walk_speed,
        AgentType::Predator => config.0.predator.walk_speed,
    };
    let run_top_speed = match agent.agent_type {
        AgentType::Prey => config.0.prey.run_speed,
        AgentType::Predator => config.0.predator.run_speed,
    };
    let mut deceleration = match agent.agent_type {
        AgentType::Prey => config.0.prey.deceleration,
        AgentType::Predator => config.0.predator.deceleration,
    };
    let turn_speed = match agent.agent_type {
        AgentType::Prey => config.0.prey.turn_speed,
        AgentType::Predator => config.0.predator.turn_speed,
    };

    let running = matches!(action, Action::Run | Action::TurnRun(_));
    let mut acceleration = if running {
        run_acceleration
    } else {
        walk_acceleration
    };
    let mut top_speed = if running {
        run_top_speed
    } else {
        walk_top_speed
    };
    if map.0.get_env_type(
        selected.1.location,
        (-half_size.x, half_size.x),
        (-half_size.y, half_size.y),
    ) == map::EnvType::Water
    {
        acceleration *= config.0.world.water_multiplier;
        top_speed *= config.0.world.water_multiplier;
        deceleration /= config.0.world.water_multiplier;
    }

    if matches!(action, Action::Eat(_)) {
        let bbox_shape_self = Vec2::new(
            config.0.predator.size,
            config.0.predator.size * config.0.predator.wl_ratio,
        );
        let bbox_shape_prey_alive = Vec2::new(
            config.0.prey.size,
            config.0.prey.size * config.0.prey.wl_ratio,
        );
        let bbox_shape_prey_dead = Vec2::new(
            config.0.prey.size,
            config.0.prey.size * config.0.prey.hl_ratio,
        );
        match agent.agent_type {
            AgentType::Prey => eat_prey(&mut agent),
            AgentType::Predator => eat_predator(
                &mut agent,
                &selected,
                preys,
                bbox_shape_self,
                bbox_shape_prey_alive,
                bbox_shape_prey_dead,
            ),
        }
    } else if matches!(action, Action::Procreate(_)) {
        let bbox_shape_prey = Vec2::new(
            config.0.prey.size,
            config.0.prey.size * config.0.prey.wl_ratio,
        );
        let bbox_shape_predator = Vec2::new(
            config.0.predator.size,
            config.0.predator.size * config.0.predator.wl_ratio,
        );
        let potential_targets = match selected.1.agent_type {
            AgentType::Prey => preys,
            AgentType::Predator => predators,
        };
        let bbox_shape_self = match selected.1.agent_type {
            AgentType::Prey => bbox_shape_prey,
            AgentType::Predator => bbox_shape_predator,
        };
        let bbox_shape_other = match selected.1.agent_type {
            AgentType::Prey => bbox_shape_prey,
            AgentType::Predator => bbox_shape_predator,
        };
        if let Some(target) = get_intersecting_agents(
            &selected,
            potential_targets,
            bbox_shape_self,
            bbox_shape_other,
        )
        .iter()
        .filter(|a| a.1.alive)
        .choose(&mut rng)
        {
            agent.action = Action::Procreate(Some(target.0));
        } else {
            agent.action = Action::Procreate(None);
        }
    }
    if matches!(
        action,
        Action::Walk | Action::Run | Action::TurnWalk(_) | Action::TurnRun(_)
    ) && agent.speed <= top_speed
    {
        agent.action = action;
        agent.speed += acceleration;
        if agent.speed > top_speed {
            agent.speed = top_speed;
        }
    } else {
        agent.speed -= deceleration;
        if agent.speed < 0.0 {
            agent.speed = 0.0;
        }
    }
    if matches!(
        action,
        Action::Turn(TurnDirection::Left)
            | Action::TurnWalk(TurnDirection::Left)
            | Action::TurnRun(TurnDirection::Left)
    ) {
        agent.action = action;
        agent.direction = (agent.direction + turn_speed) % (2.0 * PI);
    }
    if matches!(
        action,
        Action::Turn(TurnDirection::Right)
            | Action::TurnWalk(TurnDirection::Right)
            | Action::TurnRun(TurnDirection::Right)
    ) {
        agent.action = action;
        agent.direction = (agent.direction - turn_speed) % (2.0 * PI);
    }
    let direction = Vec2::new(agent.direction.cos(), -agent.direction.sin());
    let change = direction * agent.speed;
    agent.location += change;
    if agent.location.x <= -half_size.x {
        agent.location.x = -half_size.x + 1e-3;
    } else if agent.location.x >= half_size.x {
        agent.location.x = half_size.x - 1e-3;
    }
    if agent.location.y <= -half_size.y {
        agent.location.y = -half_size.y + 1e-3;
    } else if agent.location.y >= half_size.y {
        agent.location.y = half_size.y - 1e-3;
    }
    agent
}

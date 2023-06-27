use std::f32::consts::PI;

use bevy::prelude::*;
use rayon::prelude::*;

use crate::{
    assets::MapAsset,
    entities::{bbox::get_bbox_corners, AgentType},
};

use super::{intersect::seg_box_intersect, map::EnvType};
use crate::helpers::config_parser::Config;

#[derive(Clone, Copy, PartialEq, Debug, Reflect, FromReflect)]
pub enum Detection {
    PreyAlive(f32, f32), // Energy, Direction
    PreyDead(f32),
    Predator(f32, f32), // Energy, Direction
    Wall,
    None,
}
impl Detection {
    pub fn get_index(&self) -> usize {
        match self {
            Self::PreyAlive(_, _) => 0,
            Self::PreyDead(_) => 1,
            Self::Predator(_, _) => 2,
            Self::Wall => 3,
            Self::None => 4,
        }
    }
}

#[derive(Clone, Debug, Reflect, FromReflect)]
pub struct RayDetection {
    pub distance: f32,
    pub detection: Detection,
    pub food: bool,
    pub env: EnvType, // vision only
    pub direction: Vec2,
}
impl RayDetection {
    pub fn none(direction: Vec2, curr_env: EnvType) -> Self {
        Self {
            distance: -1.0,
            detection: Detection::None,
            food: false,
            env: curr_env,
            direction,
        }
    }
    pub fn is_none(&self) -> bool {
        self.distance < 0.0
    }
}

fn cast_rays(
    selected: Entity,
    t: AgentType,
    location: Vec2,
    world_borders: (Vec2, Vec2),
    map: &MapAsset,
    mut rays: Vec<RayDetection>,
    distance: f32,
    corpses: &[(Entity, Vec2, f32, f32)],
    preys: &[(Entity, Vec2, f32, f32, u32)],
    predators: &[(Entity, Vec2, f32, f32)],
    config: &Config,
) -> Vec<RayDetection> {
    let curr_env = map.0.get_env_type(
        location,
        (world_borders.0.x, world_borders.1.x),
        (world_borders.0.y, world_borders.1.y),
    );
    let forrest_vm = config.world.forrest_vision_multiplier;
    let num_checks = 10u32;
    let check_dist = distance / num_checks as f32;
    // Agents
    rays.par_iter_mut().for_each(|ray| {
        let dir = ray.direction;
        for (e, l, d, en) in corpses {
            if *e == selected {
                continue;
            }
            let corpse_bb = get_bbox_corners(
                *l,
                *d,
                Vec2::new(config.prey.size, config.prey.size * config.prey.hl_ratio),
            );
            let a_env = map.0.get_env_type(
                *l,
                (world_borders.0.x, world_borders.1.x),
                (world_borders.0.y, world_borders.1.y),
            );
            let visible_distance = if a_env == EnvType::Forest {
                forrest_vm * distance
            } else {
                distance
            };
            if let Some(intercept) =
                seg_box_intersect(location, location + distance * dir, corpse_bb)
            {
                let dist = (intercept - location).length();
                if dist <= visible_distance && (ray.is_none() || dist < ray.distance) {
                    *ray = RayDetection {
                        distance: dist,
                        detection: Detection::PreyDead(*en),
                        food: t == AgentType::Predator,
                        env: curr_env,
                        direction: ray.direction,
                    };
                }
            }
        }
        for (e, l, d, en, _) in preys {
            if *e == selected {
                continue;
            }
            let prey_bb = get_bbox_corners(
                *l,
                *d,
                Vec2::new(config.prey.size, config.prey.size * config.prey.wl_ratio),
            );
            let a_env = map.0.get_env_type(
                *l,
                (world_borders.0.x, world_borders.1.x),
                (world_borders.0.y, world_borders.1.y),
            );
            let visible_distance = if a_env == EnvType::Forest {
                forrest_vm * distance
            } else {
                distance
            };
            if let Some(intercept) = seg_box_intersect(location, location + distance * dir, prey_bb)
            {
                let dist = (intercept - location).length();
                if dist <= visible_distance && (ray.is_none() || dist < ray.distance) {
                    *ray = RayDetection {
                        distance: dist,
                        detection: Detection::PreyAlive(*en, *d),
                        food: t == AgentType::Predator,
                        env: curr_env,
                        direction: ray.direction,
                    };
                }
            }
        }
        for (e, l, d, en) in predators {
            if *e == selected {
                continue;
            }
            let predator_bb = get_bbox_corners(
                *l,
                *d,
                Vec2::new(
                    config.predator.size,
                    config.predator.size * config.predator.wl_ratio,
                ),
            );
            let a_env = map.0.get_env_type(
                *l,
                (world_borders.0.x, world_borders.1.x),
                (world_borders.0.y, world_borders.1.y),
            );
            let visible_distance = if a_env == EnvType::Forest {
                forrest_vm * distance
            } else {
                distance
            };
            if let Some(intercept) =
                seg_box_intersect(location, location + distance * dir, predator_bb)
            {
                let dist = (intercept - location).length();
                if dist <= visible_distance && (ray.is_none() || dist < ray.distance) {
                    *ray = RayDetection {
                        distance: dist,
                        detection: Detection::Predator(*en, *d),
                        food: false,
                        env: curr_env,
                        direction: ray.direction,
                    };
                }
            }
        }
        // Walls
        let mut ray_end = location + distance * dir;
        if ray.is_none()
            && (ray_end.x < world_borders.0.x
                || ray_end.x > world_borders.1.x
                || ray_end.y < world_borders.0.y
                || ray_end.y > world_borders.1.y)
        {
            if ray_end.x < world_borders.0.x {
                ray_end.x = world_borders.0.x;
            } else if ray_end.x > world_borders.1.x {
                ray_end.x = world_borders.1.x;
            }
            if ray_end.y < world_borders.0.y {
                ray_end.y = world_borders.0.y;
            } else if ray_end.y > world_borders.1.y {
                ray_end.y = world_borders.1.y;
            }
            *ray = RayDetection {
                distance: (ray_end - location).length(),
                detection: Detection::Wall,
                food: false,
                env: curr_env,
                direction: ray.direction,
            };
        }
        // Environment
        for j in 1..num_checks {
            let dist = check_dist * j as f32;
            let ray_end = location + dist * dir;
            let env = map.0.get_env_type(
                ray_end,
                (world_borders.0.x, world_borders.1.x),
                (world_borders.0.y, world_borders.1.y),
            );
            if curr_env != env {
                ray.env = env;
                if t == AgentType::Predator {
                    break;
                }
            }
            if env == EnvType::Food && t == AgentType::Prey {
                ray.food = true;
                if ray.distance < 0.0 {
                    ray.distance = dist;
                }
                if ray.env != curr_env {
                    break;
                }
            }
        }
    });
    rays
}

pub fn cast_rays_vision(
    selected: Entity,
    t: AgentType,
    location: Vec2,
    direction: f32,
    world_borders: (Vec2, Vec2),
    map: &MapAsset,
    fov: f32,
    distance: f32,
    num_rays: usize,
    corpses: &[(Entity, Vec2, f32, f32)],
    preys: &[(Entity, Vec2, f32, f32, u32)],
    predators: &[(Entity, Vec2, f32, f32)],
    config: &Config,
) -> Vec<RayDetection> {
    let mut directions = Vec::with_capacity(num_rays);
    for i in 0..num_rays {
        let angle = if num_rays > 1 {
            direction + fov * (i as f32 / (num_rays - 1) as f32 - 0.5)
        } else {
            direction
        };
        directions.push(Vec2::new(angle.cos(), -angle.sin()));
    }
    cast_rays(
        selected,
        t,
        location,
        world_borders,
        map,
        directions
            .into_iter()
            .map(|d| RayDetection::none(d, EnvType::Meadow))
            .collect::<Vec<_>>(),
        distance,
        corpses,
        preys,
        predators,
        config,
    )
}

pub fn cast_rays_hearing(
    selected: Entity,
    t: AgentType,
    location: Vec2,
    direction: f32,
    world_borders: (Vec2, Vec2),
    map: &MapAsset,
    distance: f32,
    num_rays: usize,
    corpses: &[(Entity, Vec2, f32, f32)],
    preys: &[(Entity, Vec2, f32, f32, u32)],
    predators: &[(Entity, Vec2, f32, f32)],
    config: &Config,
) -> Vec<RayDetection> {
    let mut directions = Vec::with_capacity(num_rays);
    let incr = 2.0 * PI / num_rays as f32;
    for i in 0..num_rays {
        let angle = (direction + i as f32 * incr) % (2.0 * PI);
        directions.push(Vec2::new(angle.cos(), -angle.sin()));
    }
    cast_rays(
        selected,
        t,
        location,
        world_borders,
        map,
        directions
            .into_iter()
            .map(|d| RayDetection::none(d, EnvType::Meadow))
            .collect::<Vec<_>>(),
        distance,
        corpses,
        preys,
        predators,
        config,
    )
}

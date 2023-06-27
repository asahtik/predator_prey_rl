use std::f32::consts::PI;

use bevy::{pbr::NotShadowCaster, prelude::*};
use bevy_mod_picking::{Highlighting, PickableBundle};

use rand::Rng;
use rand_distr::{Distribution, UnitCircle};

use super::{Agent, AgentType};

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    scene: &Handle<Scene>,
    scale: f32,
    bb: shape::Box,
    t: AgentType,
    loc: Vec2,
    direction: f32,
    life: usize,
) {
    commands
        .spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(bb)),
                material: materials.add(Color::rgba(0.5, 1.0, 0.5, 0.1).into()),
                transform: Transform::from_xyz(loc.x, 0.0, loc.y)
                    .with_rotation(Quat::from_rotation_y(direction)),
                visibility: Visibility::Visible,
                ..default()
            },
            NotShadowCaster,
            Agent::new(t, loc, direction, life),
        ))
        .with_children(|parent| {
            parent.spawn(SceneBundle {
                scene: scene.clone(),
                transform: Transform::from_xyz(0.0, 0.0, 0.0)
                    .with_scale(Vec3::new(scale, scale, scale)),
                visibility: Visibility::Visible,
                ..default()
            });
        })
        .insert(PickableBundle::default())
        .insert(Highlighting {
            initial: materials.add(Color::rgba(0.5, 1.0, 0.5, 0.1).into()),
            hovered: Some(materials.add(Color::rgba(0.5, 0.5, 1.0, 0.1).into())),
            pressed: None,
            selected: Some(materials.add(Color::rgba(0.5, 0.5, 0.5, 0.2).into())),
        });
}

pub fn batch_spawn(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    world_size: Vec2,
    scene: &Handle<Scene>,
    scale: f32,
    bb: shape::Box,
    t: AgentType,
    count: u32,
    batch_count: u32,
    batch_radius: f32,
    life: usize,
) {
    let mut rng = rand::thread_rng();
    let mut to_spawn = count;
    while to_spawn > 0 {
        let x = rng.gen::<f32>() * world_size.x;
        let y = rng.gen::<f32>() * world_size.y;

        // let direction = rng.gen::<f32>() * PI;
        let direction = 0.0;
        spawn(
            commands,
            meshes,
            materials,
            scene,
            scale,
            bb,
            t,
            Vec2::new(x, y) - world_size / 2.0,
            direction,
            life,
        );
        to_spawn -= 1;
        if to_spawn > 0 {
            let batch_size = rng.gen_range(0..=u32::min(batch_count, to_spawn));
            for _ in 0..batch_size {
                let [bx, by]: [f32; 2] = UnitCircle.sample(&mut rng);
                let bx = bx * batch_radius + x;
                let by = by * batch_radius + y;
                if bx < 0.0 || bx >= world_size.x || by < 0.0 || by >= world_size.y {
                    continue;
                }
                let bdirection = rng.gen::<f32>() * PI;
                spawn(
                    commands,
                    meshes,
                    materials,
                    scene,
                    scale,
                    bb,
                    t,
                    Vec2::new(bx, by) - world_size / 2.0,
                    bdirection,
                    life,
                );
                to_spawn -= 1;
            }
        }
    }
}

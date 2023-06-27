use bevy::prelude::{Entity, Vec2};

use super::Agent;

pub fn get_bbox_corners(location: Vec2, direction: f32, bbox_shape: Vec2) -> [Vec2; 4] {
    let half_shape = bbox_shape / 2.0;
    let angle = Vec2::from_angle(direction);
    [
        angle.rotate(Vec2::new(-half_shape.x, half_shape.y)) + location,
        angle.rotate(Vec2::new(half_shape.x, half_shape.y)) + location,
        angle.rotate(Vec2::new(half_shape.x, -half_shape.y)) + location,
        angle.rotate(Vec2::new(-half_shape.x, -half_shape.y)) + location,
    ]
}

pub fn get_intersecting_agents<'a>(
    agent: &(Entity, &Agent),
    agents: &[(Entity, &'a Agent)],
    bbox_shape_self: Vec2,
    bbox_shape_other: Vec2,
) -> Vec<(Entity, &'a Agent)> {
    let mut intersecting_agents = Vec::new();
    let bbox_self = get_bbox_corners(agent.1.location, agent.1.direction, bbox_shape_self);
    agents.iter().filter(|a| a.0 != agent.0).for_each(|a| {
        let bbox_other = get_bbox_corners(a.1.location, a.1.direction, bbox_shape_other);
        if super::intersect::sat2d(bbox_self, bbox_other) {
            intersecting_agents.push((a.0, a.1));
        }
    });
    intersecting_agents
}

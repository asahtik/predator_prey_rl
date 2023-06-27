use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
};

use crate::{config::ConfigRes, states::AppState};

pub struct CameraMovementPlugin;

impl Plugin for CameraMovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(camera_movement.in_set(OnUpdate(AppState::InGame)));
    }
}

#[derive(Component)]
pub struct PrimaryCamera {
    pub radius: f32,
}

fn camera_movement(
    buttons: Res<Input<MouseButton>>,
    keyboard_input: Res<Input<KeyCode>>,
    mut motion_evr: EventReader<MouseMotion>,
    mut scroll_evr: EventReader<MouseWheel>,
    mut query: Query<&mut Transform, With<PrimaryCamera>>,
    config: Res<ConfigRes>,
) {
    let move_button = MouseButton::Left;
    let rotate_button = MouseButton::Right;
    let translate_mouse_sensitivity = config.0.camera.translate_mouse_sensitivity;
    let rotate_mouse_sensitivity = config.0.camera.rotate_mouse_sensitivity;
    if let Ok(mut transform) = query.get_single_mut() {
        let mut delta = Vec2::ZERO;
        for motion in motion_evr.iter() {
            delta += motion.delta;
        }
        let cs = (
            transform.right().normalize(),
            transform.up().normalize(),
            transform.forward().normalize(),
        );
        let ty = Vec3::Y;
        let mut tx = cs.0;
        let mut tz = cs.2;
        tx.y = 0.0;
        tz.y = 0.0;
        if tz.length() < 0.1 {
            tz = cs.1;
            tz.y = 0.0;
        }
        tx = tx.normalize();
        tz = tz.normalize();

        if buttons.pressed(move_button) {
            let dx = -delta.x * translate_mouse_sensitivity;
            let dy = delta.y * translate_mouse_sensitivity;
            transform.translation += tx * dx
                + if keyboard_input.pressed(KeyCode::LShift) {
                    ty * dy
                } else {
                    tz * dy
                };
        }
        if buttons.pressed(rotate_button) {
            let ry = delta.x * rotate_mouse_sensitivity;
            let rx = delta.y * rotate_mouse_sensitivity;
            let rot_y = Quat::from_rotation_y(ry);
            let rot_x = Quat::from_axis_angle(tx, rx);
            transform.rotate(rot_y * rot_x);
        }
        for s in scroll_evr.iter() {
            let dist = s.y * config.0.camera.scroll_sensitivity;
            transform.translation += cs.2 * dist;
        }
    }
}

use deps::*;

use crate::{
    craft::{arms::*, engine::*},
    math::*,
};

use bevy::{ecs as bevy_ecs, prelude::*, reflect as bevy_reflect};
use bevy_inspector_egui::Inspectable;

#[derive(Debug, Clone, Copy)]
pub struct CurrentCraft(pub Entity);
#[derive(Debug, Clone, Copy)]
pub struct CurrentWeapon(pub Entity);

/// FIXME: this requires the camera be at the root of the heirarchy
#[derive(Debug, Component, Reflect, Inspectable)]
pub struct CraftCamera {
    /// must have a [`GlobalTransform`] component
    /// Defaults to the [`CurrentCraft`]
    pub target: Option<Entity>,
    /// In world space.
    pub default_facing: TVec3,
    pub facing_offset_radians: TVec3,
    pub position_offset: TVec3,
    pub distance: TReal,
    pub rotation_speed: TReal,
    pub auto_align: bool,
    pub align_delay: TReal,
    // align_smooth_range: TReal,
    /// In world space.
    pub facing_direction: TVec3,
    pub secs_since_manual_rot: f32,
    pub previous_focal_point: TVec3,
    pub mouse_sensetivity: f32,
}

impl Default for CraftCamera {
    fn default() -> Self {
        Self {
            default_facing: -TVec3::Z,
            // facing_offset: [0., -0.266, 0.].into(),
            facing_offset_radians: [-15. * (real::consts::PI / 180.), 0., 0.].into(),
            position_offset: TVec3::Y,
            distance: 22.,
            rotation_speed: 5.,
            auto_align: true,
            align_delay: 1.5,
            // align_smooth_range: 45.,
            facing_direction: -TVec3::Z,
            secs_since_manual_rot: Default::default(),
            previous_focal_point: Default::default(),
            target: Default::default(),
            mouse_sensetivity: -0.2,
        }
    }
}

pub fn cam_input(
    targets: Query<&GlobalTransform>,
    mut cameras: Query<(&mut CraftCamera, &mut Transform, &GlobalTransform)>,
    mut mouse_motion_events: EventReader<bevy::input::mouse::MouseMotion>,
    // mut cursor_moved_events: EventReader<CursorMoved>,
    mut mouse_wheel_events: EventReader<bevy::input::mouse::MouseWheel>,
    k_input: Res<Input<KeyCode>>,
    time: Res<Time>,
    cur_craft: Option<Res<CurrentCraft>>,
) {
    let mouse_motion = mouse_motion_events
        .iter()
        .map(|m| m.delta)
        .reduce(|m1, m2| m1 + m2)
        .unwrap_or_default();

    let mouse_wheel = mouse_wheel_events
        .iter()
        .map(|m| m.y)
        .reduce(|m1, m2| m1 + m2)
        .unwrap_or_default();

    let toggle_free_look = k_input.pressed(KeyCode::Grave);

    for (mut cam, mut xform, glob_xform) in cameras.iter_mut() {
        let target_xform = targets.get(cam.target.unwrap_or_else(|| {
            cur_craft
                .as_ref()
                .expect("CraftCamera target not set and CurrentCraft res not found")
                .0
        }));
        if target_xform.is_err() {
            tracing::error!("camera target GlobalXform not found");
            continue;
        }
        let target_xform = target_xform.unwrap();

        // update cross frame tracking data
        cam.secs_since_manual_rot = cam.secs_since_manual_rot + time.delta_seconds();
        cam.distance += mouse_wheel;
        if toggle_free_look {
            cam.auto_align = !cam.auto_align;
        }

        // if there was mouse motion
        if mouse_motion.length_squared() > 0. {
            cam.secs_since_manual_rot = 0.;

            let mouse_motion = mouse_motion * cam.mouse_sensetivity * time.delta_seconds();
            cam.facing_direction = {
                let mut new_dir = TQuat::from_axis_angle(target_xform.local_x(), mouse_motion.y)
                    * (TQuat::from_axis_angle(target_xform.local_y(), mouse_motion.x)
                        * cam.facing_direction);

                // clamp manual rotations to the pole

                // if the new direction's pointing to the unit y after
                // being offseted and rotated by the target's transform
                let mut temp = new_dir + cam.facing_offset_radians;
                temp.y = new_dir.y.abs() + cam.facing_offset_radians.y.abs();
                if 1. - (target_xform.rotation * temp).y < 0.05 {
                    // retain the old y
                    new_dir.y = cam.facing_direction.y;
                }
                new_dir.normalize()
            };
        }

        // if auto alignment is requrired
        if cam.auto_align &&
    // did the target move this frame
    ((target_xform.translation - cam.previous_focal_point).length_squared() > 0.0001) &&
    // enough time has passed
     cam.secs_since_manual_rot > cam.align_delay
        {
            // slowly slerp to the default facing
            let adjusted_rotation = target_xform.rotation * cam.default_facing;

            let cur = TQuat::looking_to(cam.facing_direction, TVec3::Y);
            let to = TQuat::looking_to(adjusted_rotation, TVec3::Y);

            cam.facing_direction = if cur.dot(to) > 0. {
                cur.slerp(to, cam.rotation_speed * time.delta_seconds()) * TVec3::Z
            } else {
                (-cur).slerp(to, cam.rotation_speed * time.delta_seconds()) * TVec3::Z
            };
        }

        let new_rot = TQuat::looking_to(
        // negate since cameras look to -Z
        -cam.facing_direction,
        // facing should be relative to the target's roll
        target_xform.up())
        //offset
        * TQuat::from_euler(
            EulerRot::XYZ,
            cam.facing_offset_radians.x,
            cam.facing_offset_radians.y,
            cam.facing_offset_radians.z,
        );
        let new_rot = new_rot.normalize();
        let new_rot = if glob_xform.rotation.dot(new_rot) > 0. {
            glob_xform
                .rotation
                .slerp(new_rot, cam.rotation_speed * time.delta_seconds())
        } else {
            (-glob_xform.rotation).slerp(new_rot, cam.rotation_speed * time.delta_seconds())
        };

        let new_pos = target_xform.translation
        // pos offset is in the target's basis
        + (target_xform.rotation * cam.position_offset)
        + ((new_rot * TVec3::Z) * cam.distance);
        let new_pos = glob_xform
            .translation
            .lerp(new_pos, cam.rotation_speed * 4. * time.delta_seconds());
        let new_global_xform =
        // base it off the old to preserve scale
        glob_xform
            .with_translation(new_pos)
            .with_rotation(new_rot);

        // let parent_xform = if parent.is_some() {
        //     xform.calc_parent_xform(glob_xform)
        // } else {
        //     GlobalTransform::identity()
        // };

        // *xform = parent_xform
        //     .inverse()
        //     .mul_transform(new_global_xform.into());

        *xform = new_global_xform.into();

        cam.previous_focal_point = target_xform.translation;
    }
}

pub fn wpn_input(
    k_input: Res<Input<KeyCode>>,
    cur_wpn: Res<CurrentWeapon>,
    mut activate_wpn_events: EventWriter<ActivateWeaponEvent>,
) {
    if k_input.pressed(KeyCode::Space) {
        activate_wpn_events.send(ActivateWeaponEvent {
            weapon_id: cur_wpn.0,
        });
    }
}

pub fn engine_input(
    k_input: Res<Input<KeyCode>>,
    cur_craft: Res<CurrentCraft>,
    mut crafts: Query<(
        &GlobalTransform,
        &mut LinearEngineState,
        &mut AngularEngineState,
        &EngineConfig,
    )>,
    cameras: Query<&CraftCamera>,
) {
    let mut linear_input = TVec3::ZERO;
    let mut angular_input = TVec3::ZERO;

    if k_input.pressed(KeyCode::W) {
        // inverse z dir since cam faces backward
        linear_input.z -= 1.;
    }
    if k_input.pressed(KeyCode::S) {
        linear_input.z += 1.;
    }
    if k_input.pressed(KeyCode::D) {
        linear_input.x += 1.;
    }
    if k_input.pressed(KeyCode::A) {
        linear_input.x -= 1.;
    }
    if k_input.pressed(KeyCode::E) {
        linear_input.y += 1.;
    }
    if k_input.pressed(KeyCode::Q) {
        linear_input.y -= 1.;
    }

    if k_input.pressed(KeyCode::Numpad8) {
        angular_input.x += 1.;
    }
    if k_input.pressed(KeyCode::Numpad5) {
        angular_input.x -= 1.;
    }
    if k_input.pressed(KeyCode::Numpad4) {
        angular_input.y += 1.;
    }
    if k_input.pressed(KeyCode::Numpad6) {
        angular_input.y -= 1.;
    }
    if k_input.pressed(KeyCode::Numpad7) {
        angular_input.z += 1.;
    }
    if k_input.pressed(KeyCode::Numpad9) {
        angular_input.z -= 1.;
    }

    let (xform, mut lin_state, mut ang_state, craft_config) = crafts
        .get_mut(cur_craft.0)
        .expect("unable to find current craft entity");

    lin_state.input = linear_input;
    //lin_state.input.z *= -1.0;
    //lin_state.input.x *= -1.0;
    lin_state.input *= craft_config.linear_v_limit;

    ang_state.input = angular_input;
    //ang_state.input.z *= -1.0;
    ang_state.input *= craft_config.angular_v_limit;

    if let Some(c) = cameras
        .iter()
        .find(|c| c.target.is_none() || c.target.unwrap() == cur_craft.0)
    {
        if !c.auto_align {
            ang_state.input += 10.
                * crate::craft::mind::boid::steering_systems::look_at(
                    xform.rotation.inverse() * c.facing_direction,
                )
                .0;
        }
    }
}

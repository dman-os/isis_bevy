use deps::*;

use crate::{
    craft::{arms::*, attire::*, *},
    math::*,
    mind::*,
};

use bevy_rapier3d::prelude::*;

use bevy_inspector_egui::Inspectable;

#[derive(Debug, Default, Reflect, Inspectable)]
pub struct PlayerMindConfig {
    auto_steer: bool,
}

pub fn player_mind(
    cur_craft: Res<CurrentCraft>,
    config: ResMut<PlayerMindConfig>,
    mut crafts: Query<(&mut boid::BoidMindDirective,)>,
) {
    let cur_craft = if let Some(entt) = &cur_craft.entt {
        *entt
    } else {
        return;
    };
    if config.is_changed() {
        if config.auto_steer {
            todo!()
        } else {
            let (mut directive,) = crafts.get_mut(cur_craft).unwrap_or_log();
            *directive = boid::BoidMindDirective::SlaveToPlayerControl;
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CurrentCraft {
    pub entt: Option<Entity>,
}

// #[derive(Debug, Clone, Copy)]
// pub struct CurrentWeapon(pub Entity);

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
    pub mouse_sensetivity: Vec2,
}

impl Default for CraftCamera {
    fn default() -> Self {
        Self {
            default_facing: -TVec3::Z,
            // facing_offset_radians: [0., 0., 0.].into(),
            facing_offset_radians: [-15. * (real::consts::PI / 180.), 0., 0.].into(),
            position_offset: TVec3::Y,
            distance: 22.,
            rotation_speed: 5.,
            auto_align: false,
            align_delay: 1.5,
            // align_smooth_range: 45.,
            facing_direction: -TVec3::Z,
            secs_since_manual_rot: Default::default(),
            previous_focal_point: Default::default(),
            target: Default::default(),
            mouse_sensetivity: [-0.2, -0.2].into(),
        }
    }
}

// FIXME: I suspect whatver's going on here is beaking bevy_debug_lines
pub fn cam_input(
    targets: Query<&GlobalTransform>,
    mut cameras: Query<(&mut CraftCamera, &mut Transform, &GlobalTransform, &Camera)>,
    mut mouse_motion_events: EventReader<bevy::input::mouse::MouseMotion>,
    mut mouse_wheel_events: EventReader<bevy::input::mouse::MouseWheel>,
    k_input: Res<Input<KeyCode>>,
    time: Res<Time>,
    cur_craft: Res<CurrentCraft>,
    // mut cursor_moved_events: EventReader<CursorMoved>,
    // windows: Res<Windows>,
) {
    let cur_craft = if let Some(entt) = &cur_craft.entt {
        *entt
    } else {
        return;
    };
    let toggle_free_look = k_input.just_released(KeyCode::Grave);
    let disable_mouse_cam = k_input.pressed(KeyCode::LControl);
    let mouse_motion = if !disable_mouse_cam {
        mouse_motion_events
            .iter()
            .map(|m| m.delta)
            .reduce(|m1, m2| m1 + m2)
            .unwrap_or_default()
    } else {
        Vec2::ZERO
    };

    let mouse_wheel = if !disable_mouse_cam {
        mouse_wheel_events
            .iter()
            .map(|m| m.y)
            .reduce(|m1, m2| m1 + m2)
            .unwrap_or_default()
    } else {
        0.
    };

    let (mut cam, mut xform, glob_xform, _bevy_cam) = cameras.single_mut();
    let target = match &cam.target {
        Some(e) => *e,
        None => cur_craft,
    };
    let target_xform = targets.get(target);

    if target_xform.is_err() {
        tracing::error!("camera target GlobalXform not found");
        return;
    }
    let target_xform = target_xform.unwrap_or_log();

    // update cross frame tracking data
    cam.secs_since_manual_rot += time.delta_seconds();
    cam.distance += mouse_wheel;
    if toggle_free_look {
        cam.auto_align = !cam.auto_align;
    }
    /* if let Some(cursor_pos_screen) = cursor_moved_events.iter().last() {
        let cursor_pos_screen = cursor_pos_screen.position;

        let view = xform.compute_matrix();
        let window = match windows.get(bevy_cam.window) {
            Some(window) => window,
            None => {
                tracing::error!("WindowId {} does not exist", bevy_cam.window);
                panic!();
            }
        };
        let screen_size = Vec2::from([window.width() as f32, window.height() as f32]);
        let projection = bevy_cam.projection_matrix;

        // 2D Normalized device coordinate cursor position from (-1, -1) to (1, 1)
        let cursor_ndc = (cursor_pos_screen / screen_size) * 2.0 - Vec2::from([1.0, 1.0]);
        let ndc_to_world: Mat4 = view * projection.inverse();
        let world_to_ndc = projection * view;
        let is_orthographic = projection.w_axis[3] == 1.0;

        // Compute the cursor position at the near plane. The bevy camera looks at -Z.
        let ndc_near = world_to_ndc.transform_point3(-Vec3::Z * bevy_cam.near).z;
        let cursor_pos_near = ndc_to_world.transform_point3(cursor_ndc.extend(ndc_near));

        // Compute the ray's direction depending on the projection used.
        let ray_direction = if is_orthographic {
            view.transform_vector3(-Vec3::Z)
        } else {
            cursor_pos_near - xform.translation
        };
    } */

    // if there was mouse motion
    if mouse_motion.length_squared() > f32::EPSILON {
        cam.secs_since_manual_rot = 0.;

        let mouse_motion = mouse_motion * cam.mouse_sensetivity * time.delta_seconds();
        cam.facing_direction = {
            let mut new_dir = TQuat::from_axis_angle(xform.local_x(), mouse_motion.y)
                * (TQuat::from_axis_angle(xform.local_y(), mouse_motion.x) * cam.facing_direction);

            // clamp manual rotations to the pole

            // if the new direction's pointing to the unit y after
            // being offseted and rotated by the target's transform
            let mut temp = (target_xform.rotation.inverse() * new_dir) + cam.facing_offset_radians;
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

    // const MAX_CAM_DIST: f32 = 10.;
    // let new_pos = new_pos.move_towards(glob_xform.translation, cam.rotation_speed);
    // + (glob_xform.translation - new_pos)
    //     .move_towards(TVec3::ZERO, cam.rotation_speed * 4. * time.delta_seconds())
    //     .clamp_length(-MAX_CAM_DIST, MAX_CAM_DIST);
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

pub fn wpn_input(
    k_input: Res<Input<KeyCode>>,
    m_button_input: Res<Input<MouseButton>>,
    cur_craft: Res<CurrentCraft>,
    crafts: Query<(&sensors::CraftWeaponsIndex,)>,
    weapons: Query<&WeaponActivationState>,
    mut activate_wpn_events: EventWriter<ActivateWeaponEvent>,
    time: Res<Time>,
) {
    if let Some(entt) = &cur_craft.entt {
        let (index,) = crafts.get(*entt).unwrap_or_log();
        if k_input.pressed(KeyCode::Space) || m_button_input.pressed(MouseButton::Left) {
            for wpn in index.entt_to_desc.keys() {
                if weapons
                    .get(*wpn)
                    .expect_or_log("CurrentWeapon has no WeaponActivationState")
                    .can_activate(&time)
                {
                    activate_wpn_events.send(ActivateWeaponEvent { weapon_id: *wpn });
                }
            }
        }
    }
}

#[derive(Debug, Clone, Reflect, Inspectable)]
pub struct PlayerEngineConfig {
    /// In local basis.
    pub set_vel: TVec3,
    pub adjust_rate: TReal,
}

impl Default for PlayerEngineConfig {
    fn default() -> Self {
        Self {
            set_vel: TVec3::ZERO,
            adjust_rate: 10.,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PlayerBoidInput {
    engine_lin: boid::steering::LinearRoutineOutput,
    engine_ang: boid::steering::AngularRoutineOutput,
}

impl PlayerBoidInput {

    /// Get the player boid input's engine lin.
    pub fn engine_lin(&self) -> boid::steering::LinearRoutineOutput {
        self.engine_lin
    }

    /// Get the player boid input's engine ang.
    pub fn engine_ang(&self) -> boid::steering::AngularRoutineOutput {
        self.engine_ang
    }
}

/* #[derive(Debug, educe::Educe)]
#[educe(Deref, DerefMut)]
pub struct LinVelPid(crate::utils::PIDControllerVec3);

impl Default for LinVelPid
{
    fn default() -> Self {
        Self(
            crate::utils::PIDControllerVec3::new(
                TVec3::ONE * 30.0,
                TVec3::ONE * 0.0,
                TVec3::ONE,
                TVec3::ONE,
                TVec3::ONE * -0.,
            )
        )
    }
} */

pub fn engine_input(
    mut player_input: ResMut<PlayerBoidInput>,
    mut player_eng_conf: ResMut<PlayerEngineConfig>,
    k_input: Res<Input<KeyCode>>,
    cur_craft: Res<CurrentCraft>,
    crafts: Query<(
        &GlobalTransform,
        &engine::LinearEngineState,
        &RigidBodyCollidersComponent,
        &boid::steering::CraftControllerConsts,
    )>,
    cameras: Query<(&GlobalTransform, &CraftCamera)>,
    query_pipeline: Res<QueryPipeline>,
    collider_query: QueryPipelineColliderComponentsQuery,
    // mut pid: Local<RotToVelPid>,
    // mut pid: Local<LinVelPid>,
) {
    let cur_craft = if let Some(entt) = &cur_craft.entt {
        *entt
    } else {
        return;
    };
    let mut linear_input = TVec3::ZERO;
    let mut angular_input = TVec3::ZERO;

    let shift_pressed = k_input.pressed(KeyCode::LShift);

    if k_input.pressed(KeyCode::W) {
        // inverse z dir since cam faces backward
        linear_input.z -= 1.;
        if shift_pressed {
            player_eng_conf.set_vel.z -= player_eng_conf.adjust_rate;
        }
    }
    if k_input.pressed(KeyCode::S) {
        linear_input.z += 1.;
        if shift_pressed {
            player_eng_conf.set_vel.z += player_eng_conf.adjust_rate;
        }
    }
    if k_input.pressed(KeyCode::D) {
        linear_input.x += 1.;
    }
    if k_input.pressed(KeyCode::A) {
        linear_input.x -= 1.;
    }
    if k_input.pressed(KeyCode::E) {
        if shift_pressed {
            angular_input.z -= 1.;
        } else {
            linear_input.y += 1.;
        }
    }
    if k_input.pressed(KeyCode::Q) {
        if shift_pressed {
            angular_input.z += 1.;
        } else {
            linear_input.y -= 1.;
        }
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
    if k_input.pressed(KeyCode::LAlt) {
        angular_input *= 0.1;
    } else {
        angular_input *= 10.;
    }

    let (xform, lin_state, craft_colliders, consts) = crafts
        .get(cur_craft)
        .expect_or_log("unable to find current craft entity");
    player_input.engine_lin = if linear_input.length_squared() > TReal::EPSILON {
        boid::steering::LinearRoutineOutput::FracAccel(xform.rotation * linear_input)
    } else {
        boid::steering::LinearRoutineOutput::Accel(
            xform.rotation
                * crate::utils::p_controller_vec3(
                    player_eng_conf.set_vel - lin_state.velocity,
                    consts.kp_vel_to_accel_lin,
                ),
        )
    };
    player_input.engine_ang = angular_input.into();

    if let Some((cam_xform, craft_cam)) = cameras
        .iter()
        .find(|(_, c)| c.target.is_none() || c.target.unwrap_or_log() == cur_craft)
    {
        if !craft_cam.auto_align {
            // Wrap the bevy query so it can be used by the query pipeline.
            let collider_set = QueryPipelineColliderComponentsSet(&collider_query);
            let ray = Ray::new(
                cam_xform.translation.into(),
                // craft_xform.translation.into(),
                craft_cam.facing_direction.normalize().into(),
            );
            let toi = match query_pipeline.cast_ray(
                &collider_set,
                &ray,
                5_000.,
                false,
                InteractionGroups::new(
                    ColliderGroups::SOLID.bits(),
                    (ColliderGroups::SOLID | ColliderGroups::CRAFT_SOLID).bits(),
                ),
                Some(&|handle| {
                    // not a craft collider
                    !craft_colliders.0 .0[..].contains(&handle)
                }),
            ) {
                Some((_, hit_toi)) => hit_toi,

                None => 5_000.,
            };
            let hit: TVec3 = ray.point_at(toi).into();
            player_input.engine_ang.0 += boid::steering::look_to(
                xform.rotation.inverse() * (hit - xform.translation).normalize(),
            );
        }
    }
}

#[derive(Component)]
pub struct CraftFwdMarker;

#[derive(Component)]
pub struct FacingMarker;

#[derive(Component)]
pub struct VelocityDirMarker;

pub fn setup_markers(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    // mut meshes: ResMut<Assets<Mesh>>,
    // mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let text_style = TextStyle {
        font: asset_server.load("fonts/BrassMono/regular_cozy.otf"),
        font_size: 25.0,
        color: Color::rgba(1., 1., 1., 0.9),
    };
    commands
        .spawn()
        .insert_bundle(TextBundle {
            text: Text {
                // Construct a `Vec` of `TextSection`s
                sections: vec![TextSection {
                    value: "+".to_string(),
                    style: text_style.clone(),
                }],
                ..Default::default()
            },
            style: Style {
                position_type: PositionType::Absolute,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(FacingMarker);

    commands
        .spawn()
        .insert_bundle(TextBundle {
            text: Text {
                // Construct a `Vec` of `TextSection`s
                sections: vec![TextSection {
                    value: "x".to_string(),
                    style: text_style.clone(),
                }],
                ..Default::default()
            },
            style: Style {
                position_type: PositionType::Absolute,
                ..Default::default()
            },
            ..Default::default()
        })
        /*
        .insert_bundle(ImageBundle {
            image: asset_server.load("textures/crosshair_simple_64.png").into(),
            style: Style {
                size: Size::new(Val::Px(64.), Val::Px(64.)),
                position_type: PositionType::Absolute,
                ..Default::default()
            },
            ..Default::default()
        }) */
        .insert(CraftFwdMarker);
    commands
        .spawn()
        .insert_bundle(TextBundle {
            text: Text {
                // Construct a `Vec` of `TextSection`s
                sections: vec![TextSection {
                    value: "[ ]".to_string(),
                    style: text_style,
                }],
                ..Default::default()
            },
            style: Style {
                position_type: PositionType::Absolute,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(VelocityDirMarker);
}

pub fn update_ui_markers(
    mut query: QuerySet<(
        QueryState<(&mut Style, &mut Visibility, &CalculatedSize), With<CraftFwdMarker>>,
        QueryState<(&mut Style, &mut Visibility, &CalculatedSize), With<VelocityDirMarker>>,
        QueryState<(&mut Style, &mut Visibility, &CalculatedSize), With<FacingMarker>>,
        QueryState<(&mut Style, &mut Visibility, &CalculatedSize), With<Crosshair>>,
    )>,
    cur_craft: Res<CurrentCraft>,
    crafts: Query<(
        &GlobalTransform,
        &CraftDimensions,
        &RigidBodyVelocityComponent,
        &RigidBodyCollidersComponent,
    )>,
    windows: Res<Windows>,
    cameras: Query<(&GlobalTransform, &Camera)>,
    active_cameras: Res<bevy::render::camera::ActiveCameras>,
    craft_cameras: Query<&CraftCamera>,
    query_pipeline: Res<QueryPipeline>,
    collider_query: QueryPipelineColliderComponentsQuery,
    weapons: Query<(&GlobalTransform, &CrosshairState)>,
) {
    let cur_craft = if let Some(entt) = &cur_craft.entt {
        *entt
    } else {
        return;
    };
    let active_cam = match active_cameras
        .get("camera_3d")
        .expect_or_log("'camera_3d' not found amongst ActiveCameras")
        .entity
    {
        Some(e) => e,
        None => return,
    };

    let (craft_xform, dim, vel, craft_colliders) = crafts
        .get(cur_craft)
        .expect_or_log("unable to find current craft entity");
    let (cam_xform, cam) = cameras
        .get(active_cam)
        .expect_or_log("unable to find 'camera_3d' entity");

    let window = windows
        .get(cam.window)
        .expect_or_log("unable to find Camera's window");
    let window_size = Vec2::new(window.width(), window.height());
    const PADDING: Vec2 = bevy::math::const_vec2!([32., 32.]);
    const MARKER_MAX_RANGE: TReal = 5_000.0;
    let world_vel: TVec3 = vel.linvel.into();

    // Wrap the bevy query so it can be used by the query pipeline.
    let collider_set = QueryPipelineColliderComponentsSet(&collider_query);

    // craft facing marker
    {
        let ray = Ray::new(
            (craft_xform.translation).into(),
            craft_xform.forward().into(),
        );
        let toi = match query_pipeline.cast_ray(
            &collider_set,
            &ray,
            MARKER_MAX_RANGE,
            false,
            InteractionGroups::new(
                ColliderGroups::SOLID.bits(),
                (ColliderGroups::SOLID | ColliderGroups::CRAFT_SOLID).bits(),
            ),
            Some(&|handle| {
                // not a craft collider
                !craft_colliders.0 .0[..].contains(&handle)
            }),
        ) {
            Some((_, hit_toi)) => hit_toi,
            None => MARKER_MAX_RANGE,
        };

        let fwd_marker_pos = cam
            .world_to_screen(&windows, cam_xform, ray.point_at(toi).into())
            .unwrap_or_default();
        let fwd_marker_pos = fwd_marker_pos.clamp(PADDING, window_size - PADDING);
        for (mut style, mut _visibility, calc_size) in query.q0().iter_mut() {
            style.position = Rect {
                left: Val::Px(fwd_marker_pos.x - (calc_size.size.width * 0.5)),
                right: Val::Px(fwd_marker_pos.x + (calc_size.size.width * 0.5)),
                top: Val::Px(fwd_marker_pos.y + (calc_size.size.height * 0.5)),
                bottom: Val::Px(fwd_marker_pos.y - (calc_size.size.height * 0.5)),
            };
        }
    }

    // craft vel marker
    if world_vel.length_squared() > 0.01 {
        let vel_marker_pos = cam
            .world_to_screen(
                &windows,
                cam_xform,
                craft_xform.translation + (world_vel.normalize() * dim.max_element() * 2.),
            )
            .unwrap_or_default();
        let vel_marker_pos = vel_marker_pos.clamp(PADDING, window_size - PADDING);

        for (mut style, mut visibility, calc_size) in query.q1().iter_mut() {
            style.position = Rect {
                left: Val::Px(vel_marker_pos.x - (calc_size.size.width * 0.5)),
                right: Val::Px(vel_marker_pos.x + (calc_size.size.width * 0.5)),
                top: Val::Px(vel_marker_pos.y + (calc_size.size.height * 0.5)),
                bottom: Val::Px(vel_marker_pos.y - (calc_size.size.height * 0.5)),
            };
            visibility.is_visible = true;
        }
    } else {
        for (_, mut visibility, _) in query.q1().iter_mut() {
            visibility.is_visible = false;
        }
    }

    // mouse direction marker
    match craft_cameras.get_single() {
        Ok(craft_cam) => {
            let ray = Ray::new(
                cam_xform.translation.into(),
                // craft_xform.translation.into(),
                craft_cam.facing_direction.normalize().into(),
            );
            let toi = match query_pipeline.cast_ray(
                &collider_set,
                &ray,
                MARKER_MAX_RANGE,
                false,
                InteractionGroups::new(
                    ColliderGroups::SOLID.bits(),
                    (ColliderGroups::SOLID | ColliderGroups::CRAFT_SOLID).bits(),
                ),
                Some(&|handle| {
                    // not a craft collider
                    !craft_colliders.0 .0[..].contains(&handle)
                }),
            ) {
                Some((_, hit_toi)) => hit_toi,

                None => MARKER_MAX_RANGE,
            };
            let facing_marker_pos = cam
                .world_to_screen(&windows, cam_xform, ray.point_at(toi).into())
                .unwrap_or_default();
            // let facing_marker_pos = facing_marker_pos.clamp(PADDING, window_size - PADDING);

            for (mut style, mut visibility, calc_size) in query.q2().iter_mut() {
                style.position = Rect {
                    left: Val::Px(facing_marker_pos.x - (calc_size.size.width * 0.5)),
                    right: Val::Px(facing_marker_pos.x + (calc_size.size.width * 0.5)),
                    top: Val::Px(facing_marker_pos.y + (calc_size.size.height * 0.5)),
                    bottom: Val::Px(facing_marker_pos.y - (calc_size.size.height * 0.5)),
                };
                visibility.is_visible = true;
            }
        }
        Err(_) => {
            for (_, mut visibility, _) in query.q2().iter_mut() {
                visibility.is_visible = false;
            }
        }
    }

    // crosshairs
    let mut q3 = query.q3();
    for (xform, crosshair) in weapons.iter() {
        let ray = Ray::new(
            (xform.translation + world_vel).into(),
            (xform.forward() * crosshair.weapon_range).into(),
        );
        let toi = match query_pipeline.cast_ray(
            &collider_set,
            &ray,
            1.0,
            false,
            InteractionGroups::new(
                ColliderGroups::SOLID.bits(),
                (ColliderGroups::SOLID | ColliderGroups::CRAFT_SOLID).bits(),
            ),
            Some(&|handle| {
                // not a craft collider
                !craft_colliders.0 .0[..].contains(&handle)
            }),
        ) {
            Some((_, hit_toi)) => hit_toi,
            None => 1.0,
        };
        let crosshair_pos = cam
            .world_to_screen(&windows, cam_xform, ray.point_at(toi).into())
            .unwrap_or_default();
        let crosshair_pos = crosshair_pos.clamp(PADDING, window_size - PADDING);
        let (mut style, mut visibility, calc_size) = q3
            .get_mut(crosshair.crosshair_entt)
            .expect_or_log("Crosshair not found for weapon");
        style.position = Rect {
            left: Val::Px(crosshair_pos.x - (calc_size.size.width * 0.5)),
            right: Val::Px(crosshair_pos.x + (calc_size.size.width * 0.5)),
            top: Val::Px(crosshair_pos.y + (calc_size.size.height * 0.5)),
            bottom: Val::Px(crosshair_pos.y - (calc_size.size.height * 0.5)),
        };
        visibility.is_visible = true;
    }
}

#[derive(Component)]
pub struct Crosshair;

#[derive(Component, Debug)]
pub struct CrosshairState {
    pub crosshair_entt: Entity,
    pub weapon_range: TReal,
}

pub fn wpn_raycaster_butler(
    mut commands: Commands,
    cur_craft: Res<CurrentCraft>,
    hairy_weapons: Query<Entity, With<CrosshairState>>,
    crafts: Query<
        (
            &sensors::CraftWeaponsIndex,
            ChangeTrackers<sensors::CraftWeaponsIndex>,
        ),
        (With<Transform>, With<GlobalTransform>),
    >,
    asset_server: Res<AssetServer>,
    crosshairs: Query<Entity, With<Crosshair>>,
) {
    if let Some(entt) = &cur_craft.entt {
        let (wpn_index, has_wpns_changed) = crafts.get(*entt).unwrap_or_log();
        // if something has changed
        if cur_craft.is_changed() || has_wpns_changed.is_changed() {
            // clean out everything and reset

            for entt in hairy_weapons.iter() {
                commands.entity(entt).remove::<CrosshairState>();
            }
            for entt in crosshairs.iter() {
                commands.entity(entt).despawn_recursive();
            }

            for (wpn, desc) in wpn_index.entt_to_desc.iter() {
                let crosshair_entt = commands
                    .spawn()
                    .insert_bundle(TextBundle {
                        text: Text {
                            sections: vec![TextSection {
                                value: "(x)".to_string(),
                                style: TextStyle {
                                    font: asset_server.load("fonts/BrassMono/regular_cozy.otf"),
                                    font_size: 25.0,
                                    color: Color::rgba(1., 1., 1., 0.9),
                                }
                                .clone(),
                            }],
                            ..Default::default()
                        },
                        style: Style {
                            position_type: PositionType::Absolute,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .insert(Crosshair)
                    .id();
                commands.entity(*wpn).insert(CrosshairState {
                    crosshair_entt,
                    weapon_range: desc.range,
                });
            }
        }
    } else {
        if cur_craft.is_changed() {
            // i.e. player has no craft
            for entt in hairy_weapons.iter() {
                commands.entity(entt).remove::<CrosshairState>();
            }
            for entt in crosshairs.iter() {
                commands.entity(entt).despawn_recursive();
            }
        }
    }
}

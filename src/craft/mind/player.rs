use deps::*;

use crate::{
    craft::{arms::*, engine::*, mind::*},
    math::*,
};

use bevy::{ecs as bevy_ecs, reflect as bevy_reflect};
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
            auto_align: true,
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

pub fn cam_input(
    targets: Query<&GlobalTransform>,
    mut cameras: Query<(&mut CraftCamera, &mut Transform, &GlobalTransform, &Camera)>,
    mut mouse_motion_events: EventReader<bevy::input::mouse::MouseMotion>,
    mut mouse_wheel_events: EventReader<bevy::input::mouse::MouseWheel>,
    k_input: Res<Input<KeyCode>>,
    time: Res<Time>,
    cur_craft: Option<Res<CurrentCraft>>,
    // mut cursor_moved_events: EventReader<CursorMoved>,
    // windows: Res<Windows>,
) {
    let mouse_motion = mouse_motion_events
        .iter()
        .map(|m| m.delta)
        .reduce(|m1, m2| m1 + m2)
        .unwrap_or_default();

    let _mouse_wheel = mouse_wheel_events
        .iter()
        .map(|m| m.y)
        .reduce(|m1, m2| m1 + m2)
        .unwrap_or_default();

    let toggle_free_look = k_input.just_released(KeyCode::Grave);

    let (mut cam, mut xform, glob_xform, _bevy_cam) = cameras.single_mut();
    let target_xform = targets.get(cam.target.unwrap_or_else(|| {
        cur_craft
            .as_ref()
            .expect("CraftCamera target not set and CurrentCraft res not found")
            .0
    }));
    if target_xform.is_err() {
        tracing::error!("camera target GlobalXform not found");
        return;
    }
    let target_xform = target_xform.unwrap();

    // update cross frame tracking data
    cam.secs_since_manual_rot += time.delta_seconds();
    // cam.distance += mouse_wheel;
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
    // FIXME: this lerp makes very little sense
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

pub fn wpn_input(
    k_input: Res<Input<KeyCode>>,
    cur_wpn: Res<CurrentWeapon>,
    weapons: Query<&WeaponActivationState>,
    mut activate_wpn_events: EventWriter<ActivateWeaponEvent>,
    time: Res<Time>,
) {
    if k_input.pressed(KeyCode::Space)
        && weapons
            .get(cur_wpn.0)
            .expect("CurrentWeapon has no WeaponActivationState")
            .can_activate(&time)
    {
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
    // mut pid: Local<RotToVelPid>,
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
            /* let current_rot = xform.rotation.to_euler(EulerRot::XYZ).into();
            let wanted_rot: TVec3 = TQuat::looking_to(c.facing_direction, xform.up())
                .to_euler(EulerRot::XYZ)
                .into();
            let drive = pid.0.update(
                current_rot,
                TVec3::new(
                    crate::math::delta_angle_radians(current_rot.x, wanted_rot.x),
                    crate::math::delta_angle_radians(current_rot.y, wanted_rot.y),
                    crate::math::delta_angle_radians(current_rot.z, wanted_rot.z),
                ),
                1.,
            );
            ang_state.input += drive; */

            // look_at_input *= 1. * (180. / crate::math::real::consts::PI);

            /* fn fun(x: TReal) -> TReal {
                const THRESHOLD: TReal = 0.01;
                if (x.abs() - 0.) < TReal::EPSILON || x.abs() > THRESHOLD {
                    x
                } else {
                    THRESHOLD * x.signum()
                    // (x * x.recip()).abs() * x.signum()
                }
            } */
            /*
            look_at_input.x = fun(look_at_input.x);
            look_at_input.y = fun(look_at_input.y);
            look_at_input.z = fun(look_at_input.z); */
            // look_at_input = look_at_input * look_at_input.recip();
            // ang_state.input += 1. * (180. / crate::math::real::consts::PI) * look_at_input;
            ang_state.input += 10.
                * crate::craft::mind::boid::steering_systems::look_to(
                    xform.rotation.inverse() * c.facing_direction,
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
    /*   commands
    .spawn()
    .insert_bundle(NodeBundle {
        color: UiColor(Color::NONE),
        style: Style {
            position_type: PositionType::Absolute,
            justify_content: JustifyContent::Center,
            align_content: AlignContent::Center,
            size: Size {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
            },
            ..Default::default()
        },
        ..Default::default()
    })
    .with_children(|parent| {
        parent.spawn().insert_bundle(TextBundle {
            text: Text {
                // Construct a `Vec` of `TextSection`s
                sections: vec![TextSection {
                    value: "+".to_string(),
                    style: TextStyle {
                        font: asset_server.load("fonts/test_font.ttf"),
                        font_size: 25.0,
                        color: Color::WHITE,
                    },
                }],
                ..Default::default()
            },
            style: Style {
                align_self: AlignSelf::Center,
                // position_type: PositionType::Absolute,
                // position: Rect {
                //     left: Val::Percent(50.),
                //     right: Val::Percent(50.),
                //     top: Val::Percent(50.),
                //     bottom: Val::Percent(50.),
                //     ..Default::default()
                // },
                ..Default::default()
            },
            ..Default::default()
        });
    })
    .insert(FacingMarker); */

    let text_style = TextStyle {
        font: asset_server.load("fonts/test_font.ttf"),
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

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_ui_markers(
    mut query: QuerySet<(
        QueryState<(&mut Style, &mut Visibility, &CalculatedSize), With<CraftFwdMarker>>,
        QueryState<(&mut Style, &mut Visibility, &CalculatedSize), With<VelocityDirMarker>>,
        QueryState<(&mut Style, &mut Visibility, &CalculatedSize), With<FacingMarker>>,
        QueryState<(&mut Style, &mut Visibility, &CalculatedSize), With<Crosshair>>,
    )>,
    cur_craft: Option<Res<CurrentCraft>>,
    crafts: Query<(
        &GlobalTransform,
        &crate::craft::engine::EngineConfig,
        &crate::craft::engine::LinearEngineState,
    )>,
    windows: Res<Windows>,
    cameras: Query<(&GlobalTransform, &Camera)>,
    active_cameras: Res<bevy::render::camera::ActiveCameras>,
    craft_cameras: Query<&CraftCamera>,
    weapons: Query<(
        &GlobalTransform,
        &RayCastSource<WpnFwdRaycaster>,
        &CrosshairEntt,
    )>,
) {
    let cur_craft = match cur_craft.as_ref() {
        Some(e) => e.0,
        None => return,
    };
    let active_cam = match active_cameras
        .get("camera_3d")
        .expect("'camera_3d' not found amongst ActiveCameras")
        .entity
    {
        Some(e) => e,
        None => return,
    };

    let (craft_xform, eng_conf, lin_state) = crafts
        .get(cur_craft)
        .expect("unable to find current craft entity");
    let (cam_xform, cam) = cameras
        .get(active_cam)
        .expect("unable to find 'camera_3d' entity");

    let window = windows
        .get(cam.window)
        .expect("unable to find Camera's window");
    let window_size = Vec2::new(window.width(), window.height());
    const PADDING: Vec2 = bevy::math::const_vec2!([32., 32.]);
    let world_vel = craft_xform.rotation * lin_state.velocity;
    {
        let fwd_marker_pos = cam
            .world_to_screen(
                &windows,
                cam_xform,
                (craft_xform.translation + world_vel) + (craft_xform.forward() * 2000000.),
            )
            .unwrap_or_default();
        let crosshair_pos = fwd_marker_pos.clamp(PADDING, window_size - PADDING);
        for (mut style, mut _visibility, calc_size) in query.q0().iter_mut() {
            style.position = Rect {
                left: Val::Px(crosshair_pos.x - (calc_size.size.width * 0.5)),
                right: Val::Px(crosshair_pos.x + (calc_size.size.width * 0.5)),
                top: Val::Px(crosshair_pos.y + (calc_size.size.height * 0.5)),
                bottom: Val::Px(crosshair_pos.y - (calc_size.size.height * 0.5)),
            };
        }
    }

    if lin_state.velocity.length_squared() > 0.01 {
        let vel_marker_pos = cam
            .world_to_screen(
                &windows,
                cam_xform,
                craft_xform.translation
                    + (world_vel.normalize() * eng_conf.extents.max_element() * 2.),
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

    match craft_cameras.get_single() {
        Ok(craft_cam) => {
            let facing_marker_pos = cam
                .world_to_screen(
                    &windows,
                    cam_xform,
                    craft_xform.translation + (craft_cam.facing_direction.normalize() * 2000000.),
                )
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

    let mut q3 = query.q3();
    for (xform, raycast_src, crosshair) in weapons.iter() {
        let crosshair_pos = match raycast_src.intersect_top() {
            Some((_, top_intersection)) => {
                // let transform_new = top_intersection.normal_ray().to_transform();
                cam.world_to_screen(&windows, cam_xform, top_intersection.position())
                    .unwrap_or_default()
            }
            None => cam
                .world_to_screen(
                    &windows,
                    cam_xform,
                    (xform.translation + world_vel) + (xform.forward() * 2000.),
                )
                .unwrap_or_default(),
        };
        let crosshair_pos = crosshair_pos.clamp(PADDING, window_size - PADDING);
        let (mut style, mut visibility, calc_size) = q3
            .get_mut(crosshair.0)
            .expect("Crosshair not found for weapon");
        style.position = Rect {
            left: Val::Px(crosshair_pos.x - (calc_size.size.width * 0.5)),
            right: Val::Px(crosshair_pos.x + (calc_size.size.width * 0.5)),
            top: Val::Px(crosshair_pos.y + (calc_size.size.height * 0.5)),
            bottom: Val::Px(crosshair_pos.y - (calc_size.size.height * 0.5)),
        };
        visibility.is_visible = true;
    }
}

pub type WpnFwdRaycaster = bevy_mod_picking::PickingRaycastSet;

#[derive(Component)]
pub struct Crosshair;

#[derive(Component)]
pub struct CrosshairEntt(pub Entity);

use bevy_mod_picking::RayCastSource;

#[allow(clippy::type_complexity)]
pub fn wpn_raycaster_butler(
    mut commands: Commands,
    cur_craft: Option<Res<CurrentCraft>>,
    raycast_source_query: Query<
        Entity,
        (With<RayCastSource<WpnFwdRaycaster>>, With<CrosshairEntt>),
    >,
    crafts: Query<
        (&CraftWeaponsIndex, ChangeTrackers<CraftWeaponsIndex>),
        (With<Transform>, With<GlobalTransform>),
    >,
    asset_server: Res<AssetServer>,
    crosshairs: Query<Entity, With<Crosshair>>,
) {
    let (has_changed, cur_craft) = match cur_craft.as_ref() {
        Some(e) => (e.is_changed() || e.is_added(), e.0),
        None => return,
    };
    let (wpn_index, has_wpns_changed) = crafts
        .get(cur_craft)
        .expect("CurrentCraft has no CraftWeaponsIndex");
    if !has_changed && !has_wpns_changed.is_changed() {
        return;
    }
    for entt in raycast_source_query.iter() {
        commands
            .entity(entt)
            .remove::<RayCastSource<WpnFwdRaycaster>>()
            .remove::<CrosshairEntt>();
    }
    for entt in crosshairs.iter() {
        commands.entity(entt).despawn_recursive();
    }
    for wpn in wpn_index.entt_to_class.keys() {
        let crosshair = commands
            .spawn()
            .insert_bundle(TextBundle {
                text: Text {
                    sections: vec![TextSection {
                        value: "(x)".to_string(),
                        style: TextStyle {
                            font: asset_server.load("fonts/test_font.ttf"),
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
        commands
            .entity(*wpn)
            .insert(CrosshairEntt(crosshair))
            .insert(RayCastSource::<WpnFwdRaycaster>::new_transform_empty());
    }
}

#[cfg(feature = "dylink")]
#[allow(unused_imports)]
#[allow(clippy::single_component_path_imports)]
use dylink;

use deps::*;

use anyhow::Result;
use bevy::{
    diagnostic::*,
    input::{keyboard::KeyboardInput, ElementState},
    prelude::*,
    render::camera::Camera,
    render::mesh::shape,
};
use bevy_egui::*;
use bevy_rapier3d::prelude::*;
use rand::prelude::*;

use math::{TReal, TVec3, *};

pub mod craft;
pub mod math;
pub mod utils;

#[bevy_main]
fn main() -> Result<()> {
    #[cfg(feature = "dylink")]
    println!("WARNING: dylink enabled");

    let mut app = App::build();
    app.add_plugins(DefaultPlugins)
        .insert_resource(WindowDescriptor {
            title: "ISIS".to_string(),
            ..Default::default()
        })
        .add_plugin(EguiPlugin)
        .add_plugin(RapierRenderPlugin)
        .add_plugin(DiagnosticsPlugin)
        // .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(EntityCountDiagnosticsPlugin)
        .add_plugin(FrameTimeDiagnosticsPlugin)
        .add_plugin(GamePlugin)
        //.add_plugin(bevy_prototype_debug_lines::DebugLinesPlugin)
        .add_plugin(bevy_inspector_egui::WorldInspectorPlugin::new());
    //.insert_resource(bevy::ecs::schedule::ReportExecutionOrderAmbiguities);
    //println!(
    //"{}",
    //bevy_mod_debugdump::schedule_graph::schedule_graph_dot(&app.app.schedule)
    //);
    app.run();

    Ok(())
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
            .add_plugin(craft::CraftsPlugin)
            .insert_resource(RapierConfiguration {
                gravity: [0.0, 0.0, 0.0].into(),
                ..Default::default()
            })
            .add_startup_system(setup_fps_display.system())
            .add_system(text_update_system.system())
            .insert_resource(CameraMovementSettings {
                angular_speed: std::f32::consts::PI / 2.,
                linear_speed: 20.0,
                shift_multiplier: 4.0,
                ..Default::default()
            })
            .add_startup_system(setup_environment.system())
            .add_startup_system(setup_world.system())
            .add_system(craft_state_display.system())
            .add_system(move_camera_system.system())
            //.add_system(tune_ai.system())
            .add_system(init_default_routines.system())
            .add_system(craft_input.system())
            .insert_resource(ClearColor(Color::BLACK));
    }
}

// A unit struct to help identify the FPS UI component, since there may be many Text components
struct FpsText;

fn setup_fps_display(mut commands: Commands, asset_server: Res<AssetServer>) {
    // UI camera
    commands.spawn_bundle(UiCameraBundle::default());
    // Rich text with multiple sections
    commands
        .spawn_bundle(TextBundle {
            style: Style {
                align_self: AlignSelf::FlexStart,
                ..Default::default()
            },
            // Use `Text` directly
            text: Text {
                // Construct a `Vec` of `TextSection`s
                sections: vec![
                    TextSection {
                        value: "FPS: ".to_string(),
                        style: TextStyle {
                            font: asset_server.load("fonts/test_font.ttf"),
                            font_size: 25.0,
                            color: Color::WHITE,
                        },
                    },
                    TextSection {
                        value: "".to_string(),
                        style: TextStyle {
                            font: asset_server.load("fonts/test_font.ttf"),
                            font_size: 25.0,
                            color: Color::GOLD,
                        },
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(FpsText);
}

fn text_update_system(diagnostics: Res<Diagnostics>, mut query: Query<&mut Text, With<FpsText>>) {
    for mut text in query.iter_mut() {
        if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(average) = fps.average() {
                // Update the value of the second section
                text.sections[1].value = format!("{:.2}", average);
            }
        }
    }
}

fn setup_environment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // light
    commands
        .spawn_bundle(LightBundle {
            // transform: Transform::from_xyz(4.0, 8.0, 4.0),
            transform: Transform::from_translation(TVec3::Z * -10_000.0)
                .looking_at(TVec3::ZERO, TVec3::Y),
            light: Light {
                range: 2_000_000.,
                intensity: 50_000. * 10_000.,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                transform: Transform::from_scale(TVec3::ONE * 500.),
                mesh: meshes.add(
                    shape::Icosphere {
                        radius: 1.,
                        ..Default::default()
                    }
                    .into(),
                ),
                material: materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    emissive: Color::BISQUE * 20.,
                    //unlit: true,
                    ..Default::default()
                }),
                ..Default::default()
            });
        });

    /*// camera
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(-20.0, 25., 20.0).looking_at(Vector3::ZERO, Vector3::Y),
            ..Default::default()
        })
        .insert(GameCamera);*/
}

pub struct CraftCamera;

pub struct CurrentCraft(pub Entity);
pub struct CurrentWeapon(pub Entity);

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let mut rng = rand::thread_rng();
    const SIZE_RANGE: TReal = 100.;
    const MASS_RANGE: TReal = 10_000.;
    const LOCATION_RANGE: TReal = 500.;
    // for _ in (0..50).into_iter() {
    for _ in (0..1).into_iter() {
        let size = rng.gen::<TReal>() * SIZE_RANGE;
        let radius = size * 0.5;
        let mass = rng.gen::<TReal>() * MASS_RANGE;
        let pos = {
            let pos: TVec3 = rng.gen::<[TReal; 3]>().into();
            let pos = pos * LOCATION_RANGE;
            [
                pos.x * if rng.gen_bool(0.5) { 1. } else { -1. },
                pos.y * if rng.gen_bool(0.5) { 1. } else { -1. },
                pos.z * if rng.gen_bool(0.5) { 1. } else { -1. },
            ]
            .into()
        };
        let mut xform = Transform::from_translation(pos);
        xform.rotate(Quat::from_rotation_ypr(
            rng.gen::<TReal>() * 360.0,
            rng.gen::<TReal>() * 360.0,
            rng.gen::<TReal>() * 360.0,
        ));

        commands
            .spawn()
            .insert_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Icosphere {
                    radius,
                    ..Default::default()
                })),
                transform: xform,
                material: materials.add(
                    Color::rgba(
                        rng.gen::<TReal>(),
                        rng.gen::<TReal>(),
                        rng.gen::<TReal>(),
                        1.,
                    )
                    .into(),
                ),
                ..Default::default()
            })
            .insert_bundle(RigidBodyBundle {
                activation: RigidBodyActivation::inactive(),
                position: pos.into(),
                ..Default::default()
            })
            .insert(RigidBodyPositionSync::Discrete)
            .insert_bundle(ColliderBundle {
                flags: ColliderFlags {
                    collision_groups: *craft::attire::OBSTACLE_COLLIDER_IGROUP,
                    ..Default::default()
                },
                shape: ColliderShape::ball(radius),
                mass_properties: ColliderMassProps::Density(
                    mass / (4. * math::real::consts::PI * radius * radius),
                ),
                ..Default::default()
            });
    }
    let ball_fighter_model = asset_server.load("models/ball_fighter.gltf#Scene0");

    // Spawn the craft
    let player_craft_id = commands
        .spawn_bundle(craft::CraftBundle {
            collider: craft::attire::CollisionDamageEnabledColliderBundle {
                collider: ColliderBundle {
                    shape: ColliderShape::ball(4.),
                    mass_properties: ColliderMassProps::Density(
                        15_000. / (4. * math::real::consts::PI * 4. * 4.),
                    ),
                    ..craft::attire::CollisionDamageEnabledColliderBundle::default_collider_bundle()
                },
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            // the model
            parent
                .spawn_bundle((
                    Transform::from_rotation(Quat::from_rotation_y(math::real::consts::PI)),
                    GlobalTransform::default(),
                ))
                .with_children(|parent| {
                    parent.spawn_scene(ball_fighter_model.clone());
                });

            parent.spawn_bundle(craft::attire::AttireBundle {
                profile: craft::attire::AttireProfile {
                    ..Default::default()
                },
                collider: ColliderBundle {
                    shape: ColliderShape::ball(4.),
                    ..craft::attire::AttireBundle::default_collider_bundle()
                },
            });

            let mut cam = PerspectiveCameraBundle {
                transform: Transform::from_xyz(0.0, 7., 20.0).looking_at(-TVec3::Z, TVec3::Y),
                ..Default::default()
            };
            cam.perspective_projection.far = 10_000.;
            parent.spawn_bundle(cam).insert(CraftCamera);
        })
        .id();

    commands.insert_resource(CurrentCraft(player_craft_id));

    let wpn_id = commands
        .spawn()
        .insert(craft::arms::ProjectileWeapon {
            proj_damage: craft::attire::Damage {
                value: 100.,
                damage_type: craft::attire::DamageType::Kinetic,
            },
            proj_mesh: meshes.add(
                shape::Icosphere {
                    radius: 0.5,
                    ..Default::default()
                }
                .into(),
            ),
            proj_mtr: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: Color::GOLD * 20.,
                unlit: true,
                ..Default::default()
            }),
            proj_shape: ColliderShape::ball(0.5),
            proj_velocity: TVec3::Z * -750.,
            proj_lifespan_secs: 3.,
            proj_spawn_offset: TVec3::Z * -2.,
            proj_mass: ColliderMassProps::Density(0.25 / (4. * math::real::consts::PI * 0.5 * 0.5)),
        })
        .insert_bundle(PbrBundle {
            mesh: meshes.add(
                shape::Cube {
                    size: 1.,
                    ..Default::default()
                }
                .into(),
            ),
            transform: {
                let mut t = Transform::from_translation(TVec3::Y * 3.);
                t.scale = [1., 1., 4.].into();
                t
            },
            material: materials.add(Color::WHITE.into()),
            ..Default::default()
        })
        .insert(Parent(player_craft_id))
        .id();
    commands.insert_resource(CurrentWeapon(wpn_id));

    //for ii in -7..=7 {
    for ii in 0..1 {
        commands
            .spawn()
            .insert_bundle(craft::CraftBundle {
                config: craft::engine::EngineConfig {
                    // linear_thruster_force: [0.; 3].into(),
                    ..Default::default()
                },
                rigid_body: RigidBodyBundle {
                    position: [25. * ii as TReal, 0., -50.].into(),
                    ..craft::CraftBundle::default_rb_bundle()
                },
                collider: craft::attire::CollisionDamageEnabledColliderBundle {
                    collider: ColliderBundle {
                    shape: ColliderShape::ball(4.),
                    mass_properties: ColliderMassProps::Density(
                    15_000. / (4. * math::real::consts::PI * 4. * 4.),
                    ),
                    ..craft::attire::CollisionDamageEnabledColliderBundle::default_collider_bundle()
                    },
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert_bundle(craft::mind::CraftMindBundle {
                ..Default::default()
            })
            .with_children(|parent| {
                parent
                    .spawn_bundle((
                        Transform::from_rotation(Quat::from_rotation_y(math::real::consts::PI)),
                        GlobalTransform::default(),
                    ))
                    .with_children(|parent| {
                        parent.spawn_scene(ball_fighter_model.clone());
                    });

                parent.spawn_bundle(craft::attire::AttireBundle {
                    profile: craft::attire::AttireProfile {
                        ..Default::default()
                    },
                    collider: ColliderBundle {
                        shape: ColliderShape::ball(4.),
                        ..craft::attire::AttireBundle::default_collider_bundle()
                    },
                });
            });
    }
}

pub fn init_default_routines(
    mut commands: Commands,
    player: Res<CurrentCraft>,
    crafts: Query<
        Entity,
        (
            With<craft::mind::MindConfig>,
            Without<craft::mind::ActiveRoutines>,
        ),
    >,
) {
    //return;
    let members: smallvec::SmallVec<[Entity; 8]> = crafts.iter().collect();
    if members.len() == 0 {
        // bail if there are no new crafts
        return;
    }
    tracing::info!("setting up routines");
    let group = commands
        .spawn_bundle((
            craft::mind::GroupMind {
                // add all new crafts into a new group
                members,
            },
            craft::mind::BoidFlock::default(),
        ))
        .id();
    for craft in crafts.iter() {
        let avoid_collision = commands
            .spawn_bundle(
                craft::mind::steering_systems::AvoidCollisionRoutineBundle::new(
                    craft::mind::steering_systems::AvoidCollision {
                        craft_entt: craft,
                        fwd_prediction_secs: 5.0,
                        raycast_exclusion: Default::default(),
                    },
                ),
            )
            .id();
        let active_routine = commands
            .spawn_bundle(craft::mind::steering_systems::InterceptRoutineBundle::new(
                craft::mind::steering_systems::Intercept {
                    craft_entt: craft,
                    quarry_rb: player.0.handle(),
                },
            ))
            .id();
        /*let active_routine = commands
        .spawn_bundle(
            craft::mind::steering_systems::FlyWithFlockRoutineBundle::new(
                craft::mind::steering_systems::FlyWithFlock { craft_entt: craft },
            ),
        )
        .id();*/
        commands
            .entity(craft)
            .insert(craft::mind::CraftGroup(group))
            .insert(craft::mind::ActiveRoutines::PriorityOverride {
                routines: smallvec::smallvec![
                    // avoid_collision,
                    active_routine
                ],
            });
    }
}

fn craft_input(
    k_input: Res<Input<KeyCode>>,
    cur_craft: Res<CurrentCraft>,
    cur_wpn: Res<CurrentWeapon>,
    mut crafts: Query<(
        &mut craft::engine::LinearEngineState,
        &mut craft::engine::AngularEngineState,
        &craft::engine::EngineConfig,
    )>,
    mut activate_wpn_events: EventWriter<craft::arms::ActivateWeaponEvent>,
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

    let (mut lin_state, mut ang_state, craft_config) = crafts
        .get_mut(cur_craft.0)
        .expect("unable to find current craft entity");

    lin_state.input = linear_input;
    //lin_state.input.z *= -1.0;
    //lin_state.input.x *= -1.0;
    lin_state.input *= craft_config.linear_v_limit;

    ang_state.input = angular_input;
    //ang_state.input.z *= -1.0;
    ang_state.input *= craft_config.angular_v_limit;

    if k_input.pressed(KeyCode::Space) {
        activate_wpn_events.send(craft::arms::ActivateWeaponEvent {
            weapon_id: cur_wpn.0,
        });
    }
}

fn craft_state_display(
    egui_context: ResMut<EguiContext>,
    cur_craft: Res<CurrentCraft>,
    mut crafts: Query<(
        &Transform,
        &craft::engine::LinearEngineState,
        &craft::engine::AngularEngineState,
        &mut craft::engine::LinearDriverPid,
        &mut craft::engine::AngularDriverPid,
    )>,
) {
    let (craft_xform, lin_state, ang_state, mut lin_pid, mut ang_pid) =
        crafts.get_mut(cur_craft.0).unwrap();
    egui::Window::new("Status")
        .collapsible(true)
        .default_pos([1100., 0.])
        .show(egui_context.ctx(), |ui| {
            ui.label(format!("position:      {:+03.1?}", craft_xform.translation));
            ui.label(format!("linear vel:    {:+03.1?}", lin_state.velocity));
            ui.label(format!("linear input:  {:+03.1?}", lin_state.input));
            ui.label(format!("linear flame:  {:+03.1?}", lin_state.flame));
            ui.label(format!("angular vel:   {:+03.1?}", ang_state.velocity));
            ui.label(format!("angular input: {:+03.1?}", ang_state.input));
            ui.label(format!("angular flame: {:+03.1?}", ang_state.flame));

            return;
            ui.separator();
            ui.label("linear pid tune");
            {
                let mut proportional_gain = lin_pid.0.proportional_gain.x;
                ui.add(egui::Slider::new(&mut proportional_gain, 0.0..=10_000.).text("p gain"));
                lin_pid.0.proportional_gain = [proportional_gain; 3].into();
            }

            {
                let mut integral_gain = lin_pid.0.integrat_gain.x;
                ui.add(egui::Slider::new(&mut integral_gain, 0.0..=1.).text("i gain"));
                lin_pid.0.integrat_gain = [integral_gain; 3].into();
            }

            {
                let mut differntial_gain = lin_pid.0.differntial_gain.x;
                ui.add(egui::Slider::new(&mut differntial_gain, 0.0..=1000.).text("d gain"));
                lin_pid.0.differntial_gain = [differntial_gain; 3].into();
            }

            ui.separator();
            ui.label("angular pid tune");
            {
                let mut proportional_gain = ang_pid.0.proportional_gain.x;
                ui.add(egui::Slider::new(&mut proportional_gain, 0.0..=10_000.).text("p gain"));
                ang_pid.0.proportional_gain = [proportional_gain; 3].into();
            }

            {
                let mut integral_gain = ang_pid.0.integrat_gain.x;
                ui.add(egui::Slider::new(&mut integral_gain, 0.0..=1.).text("i gain"));
                ang_pid.0.integrat_gain = [integral_gain; 3].into();
            }

            {
                let mut differntial_gain = ang_pid.0.differntial_gain.x;
                ui.add(egui::Slider::new(&mut differntial_gain, 0.0..=1000.).text("d gain"));
                ang_pid.0.differntial_gain = [differntial_gain; 3].into();
            }
            //ui.label(format!("lnear pid: {:+03.1?}", lin_pid));
            //ui.label(format!("angular pid: {:+03.1?}", ang_pid));
        });
}
fn tune_ai(
    egui_context: ResMut<EguiContext>,
    mut crafts: Query<(
        &Transform,
        &mut craft::engine::LinearEngineState,
        &mut craft::engine::AngularEngineState,
        &mut craft::engine::LinearDriverPid,
        &mut craft::engine::AngularDriverPid,
    )>,
) {
    for (_craft_xform, lin_state, ang_state, mut _lin_pid, mut ang_pid) in crafts.iter_mut() {
        egui::Window::new("mind tune")
            .collapsible(true)
            .show(egui_context.ctx(), |ui| {
                ui.label(format!("linear vel:    {:+03.1?}", lin_state.velocity));
                ui.label(format!("linear input:  {:+03.1?}", lin_state.input));
                ui.label(format!("linear flame:  {:+03.1?}", lin_state.flame));
                ui.label(format!("angular vel:   {:+03.1?}", ang_state.velocity));
                ui.label(format!("angular input: {:+03.1?}", ang_state.input));
                ui.label(format!("angular flame: {:+03.1?}", ang_state.flame));

                ui.separator();
                ui.label("angular pid tune");
                {
                    let mut proportional_gain = ang_pid.0.proportional_gain.x;
                    ui.add(egui::Slider::new(&mut proportional_gain, 0.0..=10_000.).text("p gain"));
                    ang_pid.0.proportional_gain = [proportional_gain; 3].into();
                }

                {
                    let mut integral_gain = ang_pid.0.integrat_gain.x;
                    ui.add(egui::Slider::new(&mut integral_gain, 0.0..=1.).text("i gain"));
                    ang_pid.0.integrat_gain = [integral_gain; 3].into();
                }

                {
                    let mut differntial_gain = ang_pid.0.differntial_gain.x;
                    ui.add(egui::Slider::new(&mut differntial_gain, 0.0..=1000.).text("d gain"));
                    ang_pid.0.differntial_gain = [differntial_gain; 3].into();
                }
                //ui.label(format!("lnear pid: {:+03.1?}", lin_pid));
                //ui.label(format!("angular pid: {:+03.1?}", ang_pid));
            });
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GameCamera;

#[derive(Debug, Clone, Copy, Default)]
pub struct CameraMovementSettings {
    linear_speed: TReal,
    angular_speed: TReal,
    shift_multiplier: TReal,
    linear_input: TIVec3,
    angular_input: TIVec3,
    shift_on: bool,
}

fn move_camera_system(
    mut key_events: EventReader<KeyboardInput>,
    mut cameras: Query<&mut Transform, (With<Camera>, With<GameCamera>)>,
    time: Res<Time>,
    mut cam_settings: ResMut<CameraMovementSettings>,
) {
    {
        let mut linear_input = cam_settings.linear_input;
        let mut angular_input = cam_settings.angular_input;
        let mut shift_on = cam_settings.shift_on;

        for event in key_events.iter() {
            let amount = match event.state {
                ElementState::Pressed => 1,
                ElementState::Released => -1,
            };
            if let Some(key) = event.key_code {
                match key {
                    // inverse z dir since cam faces backward
                    KeyCode::W => linear_input.z -= amount,
                    KeyCode::S => linear_input.z += amount,
                    KeyCode::D => linear_input.x += amount,
                    KeyCode::A => linear_input.x -= amount,
                    KeyCode::E => linear_input.y += amount,
                    KeyCode::Q => linear_input.y -= amount,
                    KeyCode::Numpad8 => angular_input.x += amount,
                    KeyCode::Numpad5 => angular_input.x -= amount,
                    KeyCode::Numpad4 => angular_input.y += amount,
                    KeyCode::Numpad6 => angular_input.y -= amount,
                    KeyCode::Numpad7 => angular_input.z += amount,
                    KeyCode::Numpad9 => angular_input.z -= amount,
                    KeyCode::LShift => shift_on = !shift_on,
                    _ => {}
                }
            }
        }

        cam_settings.linear_input = linear_input.clamp(-TIVec3::ONE, TIVec3::ONE);
        cam_settings.angular_input = angular_input.clamp(-TIVec3::ONE, TIVec3::ONE);
        cam_settings.shift_on = shift_on;
    }

    let mut linear_speed = cam_settings.linear_speed;

    if cam_settings.shift_on {
        linear_speed *= cam_settings.shift_multiplier
    }

    let delta_t = time.delta_seconds_f64() as TReal;
    let linear_vel = cam_settings.linear_input.as_f32() * (linear_speed * delta_t);
    let angular_vel = cam_settings.angular_input.as_f32() * (cam_settings.angular_speed * delta_t);

    // tracing::info!("linear_vel: {}, angular_vel: {}", linear_vel, angular_vel);

    let rotator = Quat::from_rotation_ypr(angular_vel.y, angular_vel.x, angular_vel.z);
    for mut camera_xform in cameras.iter_mut() {
        let cam_rotation = camera_xform.rotation;
        camera_xform.translation += cam_rotation * linear_vel;
        camera_xform.rotation *= rotator;
        // tracing::info!("resulting xform: {:?}", camera_xform);
    }
}

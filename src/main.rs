#[cfg(feature = "dylink")]
#[allow(unused_imports, clippy::single_component_path_imports)]
use dylink;

use deps::*;

use bevy::{
    diagnostic::*,
    input::{keyboard::KeyboardInput, ElementState},
    prelude::*,
    render::camera::Camera,
    render::mesh::shape,
};
use bevy_egui::*;
use bevy_prototype_debug_lines::*;
use bevy_rapier3d::prelude::*;
use rand::prelude::*;

use math::{TReal, TVec3, *};

pub mod craft;
pub mod math;
pub mod mind;
pub mod utils;

// pub struct ConsoleLog {}
// impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for ConsoleLog {}

fn main() {
    #[cfg(feature = "dylink")]
    println!("WARNING: dylink enabled");

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var(
            "RUST_LOG",
            "info,isis=info,bevy_render=info,bevy_app=info,event=info,wgpu=warn,naga=info",
        );
    }

    // let log_output = LogOutput::default();

    tracing_subscriber::fmt()
        .pretty()
        // .compact()
        // .with_ansi(false)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_timer(tracing_subscriber::fmt::time::uptime())
        // .with_writer(log_output.clone())
        .init();

    let mut inspect_registry = bevy_inspector_egui::InspectableRegistry::default();
    inspect_registry.register_raw::<RigidBodyPositionComponent, _>(|cmp, ui, _ctx| {
        ui.label(format!("{:#?}", cmp.0));
        false
    });
    inspect_registry.register_raw::<RigidBodyTypeComponent, _>(|cmp, ui, _ctx| {
        ui.label(format!("{:#?}", cmp.0));
        false
    });
    inspect_registry.register_raw::<ColliderPositionComponent, _>(|cmp, ui, _ctx| {
        ui.label(format!("{:#?}", cmp.0));
        false
    });
    inspect_registry.register_raw::<ColliderTypeComponent, _>(|cmp, ui, _ctx| {
        ui.label(format!("{:#?}", cmp.0));
        false
    });
    inspect_registry.register_raw::<mind::sensors::CraftWeaponsIndex, _>(|cmp, ui, _ctx| {
        ui.label(format!("{cmp:#?}",));
        false
    });
    inspect_registry.register_raw::<mind::player::CraftCamera, _>(|cmp, ui, _ctx| {
        ui.label(format!("{cmp:#?}",));
        false
    });
    inspect_registry.register_raw::<mind::flock::FlockMembers, _>(|cmp, ui, _ctx| {
        ui.label(format!("{cmp:#?}",));
        false
    });
    let mut app = App::new();
    app.add_plugins_with(DefaultPlugins, |group| {
        group.disable::<bevy::log::LogPlugin>()
    })
    .insert_resource(WindowDescriptor {
        title: "ISIS".to_string(),
        ..Default::default()
    })
    // .insert_resource(log_output)
    .add_plugin(EguiPlugin)
    .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
    .insert_resource(RapierConfiguration {
        gravity: [0.0, 0.0, 0.0].into(),
        ..Default::default()
    })
    .add_plugin(RapierRenderPlugin)
    .add_plugin(DiagnosticsPlugin)
    // .add_plugin(LogDiagnosticsPlugin::default())
    .add_plugin(EntityCountDiagnosticsPlugin)
    .add_plugin(FrameTimeDiagnosticsPlugin)
    .insert_resource(inspect_registry)
    .insert_resource(bevy_inspector_egui::WorldInspectorParams {
        ..Default::default()
    })
    .add_plugin(bevy_inspector_egui::WorldInspectorPlugin::new())
    .add_plugin(bevy_polyline::PolylinePlugin)
    // .add_plugins(bevy_mod_picking::DefaultPickingPlugins)
    .add_plugin(bevy_mod_picking::PickingPlugin)
    // .add_plugin(bevy_mod_picking::DebugCursorPickingPlugin)
    // .add_plugin(bevy_prototype_debug_lines::DebugLinesPlugin)
    // .insert_resource(bevy::ecs::schedule::ReportExecutionOrderAmbiguities)
    .add_plugin(GamePlugin);
    //println!(
    //"{}",
    //bevy_mod_debugdump::schedule_graph::schedule_graph_dot(&app.app.schedule)
    //);
    app.run()
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(craft::CraftsPlugin)
            .add_plugin(mind::MindPlugin)
            .add_startup_system(setup_fps_display)
            .add_system(text_update_system)
            .add_system(move_camera_system)
            // .add_system(quake_log)
            .insert_resource(CameraMovementSettings {
                angular_speed: std::f32::consts::PI / 2.,
                linear_speed: 20.0,
                shift_multiplier: 4.0,
                ..Default::default()
            })
            .add_startup_system(setup_environment)
            .add_startup_system(setup_world)
            .add_system(craft_state_display)
            .add_plugin(DebugLinesPlugin::with_depth_test(true))
            // .add_system(init_default_routines)
            // .add_startup_system(my_system)
            .insert_resource(ClearColor(Color::BLACK));
    }
}

// A unit struct to help identify the FPS UI component, since there may be many Text components
#[derive(Component)]
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

/*

#[derive(Default, Clone)]
pub struct LogOutput {
    vec: std::sync::Arc<parking_lot::RwLock<Vec<String>>>,
}

// pub struct CustomWriter<'a, T>(parking_lot::RwLockWriteGuard<'a, T>);
pub struct CustomWriter<'a>(parking_lot::RwLockWriteGuard<'a, Vec<String>>);

// impl<T> std::io::Write for CustomWriter<'_, T>
// where
//     T: std::io::Write,
// {
impl std::io::Write for CustomWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match std::io::stdout().write(buf) {
            Ok(bytes) => {
                let _ = self
                    .0
                    .push(String::from_utf8_lossy(&buf[0..bytes]).into_owned());
                Ok(bytes)
            }
            err => err,
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        std::io::stdout().flush()
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogOutput {
    // type Writer = CustomWriter<'a, Vec<u8>>;
    type Writer = CustomWriter<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        CustomWriter(self.vec.write())
    }
}

fn quake_log(
    mut egui_context: ResMut<EguiContext>,
    log_output: Res<LogOutput>,
    windows: Res<Windows>,
) {
    let (default_width, default_height) = if let Some(w) = windows.get_primary() {
        (w.width() * 0.66, w.height() * 0.15)
    } else {
        (500., 500.)
    };
    egui::Window::new("log")
        .collapsible(true)
        // .fixed_size([default_width, default_height])
        .anchor(egui::Align2::CENTER_TOP, [0., 0.])
        .default_width(default_width)
        .default_height(default_height)
        .show(egui_context.ctx_mut(), |ui| {
            let vec = log_output.vec.read();
            egui::ScrollArea::vertical()
                .always_show_scroll(true)
                .stick_to_bottom()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for row in vec.iter() {
                        ui.monospace(row);
                        ui.separator();
                    }
                });
        });
} */

fn setup_environment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // light
    commands
        .spawn_bundle(DirectionalLightBundle {
            // transform: Transform::from_xyz(4.0, 8.0, 4.0),
            transform: Transform::from_translation(TVec3::Z * -10_000.0)
                .looking_at(TVec3::ZERO, TVec3::Y),
            directional_light: DirectionalLight {
                illuminance: 100_000.,
                shadows_enabled: true,
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

    /*
    // camera
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(-20.0, 25., 20.0).looking_at(TVec3::ZERO, TVec3::Y),
            perspective_projection: PerspectiveProjection{
                far: 10_000.,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(GameCamera);
    */
}

/* fn my_system(
    mut commands: Commands,
    mut polylines: ResMut<Assets<bevy_polyline::Polyline>>,
    mut polyline_materials: ResMut<Assets<bevy_polyline::PolylineMaterial>>,
) {
    const RAY_COUNT: usize = 100;
    let RAY_DIRECTIONS = {
        let mut directions = [TVec3::ZERO; RAY_COUNT];
        let golden_ratio = (1.0 + (5.0 as TReal).sqrt()) * 0.5;
        let angle_increment = real::consts::TAU * golden_ratio;
        for ii in 0..RAY_COUNT {
            let t = ii as TReal / RAY_COUNT as TReal;
            let inclination = (1.0 - (2.0 * t)).acos();
            let azimuth = angle_increment * (ii as TReal);
            directions[ii] = TVec3::new(
                inclination.sin() * azimuth.cos(),
                inclination.sin() * azimuth.sin(),
                inclination.cos(),
            )
            .normalize();
        }
        directions
    };
    for ray in RAY_DIRECTIONS {
        commands.spawn_bundle(bevy_polyline::PolylineBundle {
            polyline: polylines.add(bevy_polyline::Polyline {
                vertices: vec![TVec3::ZERO, ray],
                ..Default::default()
            }),
            material: polyline_materials.add(bevy_polyline::PolylineMaterial {
                width: 3.0,
                color: Color::RED,
                perspective: false,
                ..Default::default()
            }),
            ..Default::default()
        });
    }
} */

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let mut rng = rand::thread_rng();

    // setup the random floating spheres
    {
        const SIZE_RANGE: TReal = 100.;
        const MASS_RANGE: TReal = 1000.;
        //const LOCATION_RANGE: [TReal; 3]= [500.; 3];
        const LOCATION_RANGE: [TReal; 3] = [500., 100.0, 500.0];
        for ii in 0..100 {
            //for _ in (0..1).into_iter() {
            let size = rng.gen::<TReal>() * SIZE_RANGE;
            let radius = size * 0.5;
            let mass = rng.gen::<TReal>() * MASS_RANGE;
            let pos = {
                let pos: TVec3 = rng.gen::<[TReal; 3]>().into();
                let pos = pos * TVec3::from(LOCATION_RANGE);
                [
                    pos.x * if rng.gen_bool(0.5) { 1. } else { -1. },
                    pos.y * if rng.gen_bool(0.5) { 1. } else { -1. },
                    pos.z * if rng.gen_bool(0.5) { 1. } else { -1. },
                ]
                .into()
            };
            let mut xform = Transform::from_translation(pos);
            xform.rotate(TQuat::from_euler(
                EulerRot::YXZ,
                rng.gen::<TReal>() * 360.0,
                rng.gen::<TReal>() * 360.0,
                rng.gen::<TReal>() * 360.0,
            ));

            commands
                .spawn()
                .insert(Name::new(format!("ball {ii}")))
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
                .insert_bundle(bevy_mod_picking::PickableBundle::default())
                /*
                .insert_bundle(RigidBodyBundle {
                    body_type: RigidBodyType::Dynamic.into(),
                    activation: RigidBodyActivation::inactive().into(),
                    position: RigidBodyPositionComponent(pos.into()),
                    ..Default::default()
                })
                .insert(RigidBodyPositionSync::Discrete)
                // */
                .insert(ColliderPositionSync::Discrete)
                .insert_bundle(ColliderBundle {
                    material: ColliderMaterial {
                        ..Default::default()
                    }
                    .into(),
                    position: ColliderPositionComponent(pos.into()),
                    flags: ColliderFlags {
                        collision_groups: *craft::attire::OBSTACLE_COLLIDER_IGROUP,
                        ..Default::default()
                    }
                    .into(),
                    shape: ColliderShape::ball(radius).into(),
                    mass_properties: ColliderMassProps::Density(
                        mass / (4. * math::real::consts::PI * radius * radius),
                    )
                    .into(),
                    ..Default::default()
                });
        }
    }

    /*  // spawn the single floating sphere
    {
        let radius = 10.;
        let pos = TVec3::new(500., 0., 500.);
        let xform = Transform::from_translation(pos);
        let mass = 10_000.;
        commands
            .spawn()
            .insert(Name::new("single_ball"))
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
            /*
            .insert_bundle(RigidBodyBundle {
                body_type: RigidBodyType::Dynamic.into(),
                activation: RigidBodyActivation::inactive().into(),
                position: RigidBodyPositionComponent(pos.into()),
                ..Default::default()
            })
            .insert(RigidBodyPositionSync::Discrete)
            // */
            .insert(ColliderPositionSync::Discrete)
            .insert_bundle(ColliderBundle {
                material: ColliderMaterial {
                    ..Default::default()
                }
                .into(),
                position: ColliderPositionComponent(pos.into()),
                flags: ColliderFlags {
                    collision_groups: *craft::attire::OBSTACLE_COLLIDER_IGROUP,
                    ..Default::default()
                }
                .into(),
                shape: ColliderShape::ball(radius).into(),
                mass_properties: ColliderMassProps::Density(
                    mass / (4. * math::real::consts::PI * radius * radius),
                )
                .into(),
                ..Default::default()
            })
            .insert_bundle(bevy_mod_picking::PickableBundle::default());
    } */

    // setup the test circuit
    let initial_point = {
        let material = materials.add(Color::PINK.into());
        let mesh = meshes.add(Mesh::from(shape::Icosphere {
            radius: 10.0,
            ..Default::default()
        }));
        #[allow(clippy::unnecessary_cast)]
        let points = [
            [1000.0, 0., 1000.0 as TReal].into(),
            //[-1000.0, 0., 1000.0].into(),
            [-1000.0, 0., -1000.0].into(),
            //[1000.0, 0., -1000.0].into(),
        ];
        let points = points.map(|p| {
            commands
                .spawn()
                .insert_bundle(bevy_mod_picking::PickableBundle::default())
                .insert_bundle(PbrBundle {
                    mesh: mesh.clone(),
                    material: material.clone(),
                    ..Default::default()
                })
                .insert_bundle(ColliderBundle {
                    flags: ColliderFlags {
                        collision_groups: *craft::attire::SENSOR_COLLIDER_IGROUP,
                        ..Default::default()
                    }
                    .into(),
                    collider_type: ColliderType::Sensor.into(),
                    shape: ColliderShape::ball(10.).into(),
                    position: (p, TQuat::IDENTITY).into(),
                    ..Default::default()
                })
                .insert(ColliderPositionSync::Discrete)
                .id()
        });
        for ii in 0..points.len() {
            let entt = points[ii];
            commands
                .entity(entt)
                .insert(Name::new(format!("waypoint {ii}")))
                .insert(mind::boid::strategy::run_circuit::CircuitWaypoint {
                    next_point: points[(ii + 1) % points.len()],
                });
        }
        points[0]
    };

    let ball_fighter_model = asset_server.load("models/ball_fighter.gltf#Scene0");
    let proj_mesh = meshes.add(
        shape::Icosphere {
            radius: 0.5,
            ..Default::default()
        }
        .into(),
    );
    let proj_mtr = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: Color::GOLD * 20.,
        unlit: true,
        ..Default::default()
    });
    let new_kinetic_cannon: &dyn Fn(_) -> _ = &(move |craft_entt| {
        craft::arms::WeaponBundle::new(
            craft::arms::ProjectileWeapon {
                proj_damage: craft::attire::Damage {
                    value: 100.,
                    damage_type: craft::attire::DamageType::Kinetic,
                },
                proj_mesh: proj_mesh.clone(),
                proj_mtr: proj_mtr.clone(),
                proj_shape: ColliderShape::ball(0.5),
                proj_velocity: TVec3::Z * -500.,
                proj_lifespan_secs: 3.,
                proj_spawn_offset: TVec3::Z * -5.,
                proj_mass: ColliderMassProps::Density(
                    0.25 / (4. * math::real::consts::PI * 0.5 * 0.5),
                ),
            },
            craft_entt,
            "kinetic_cannon",
            craft::arms::WeaponActivationState::new_discrete(5.),
        )
    });

    use mind::*;
    // spawn the player craft
    {
        let player_craft_id = commands
            .spawn()
            .insert(Name::new("player"))
            .insert_bundle(craft::CraftBundle {
                collider: craft::attire::CollisionDamageEnabledColliderBundle {
                    collider: ColliderBundle {
                        shape: ColliderShape::ball(4.).into(),
                        mass_properties: ColliderMassProps::Density(
                            15_000. / (4. * math::real::consts::PI * 4. * 4.),
                        )
                        .into(),
                        ..craft::attire::CollisionDamageEnabledColliderBundle::default_collider_bundle()
                    },
                    ..Default::default()
                },
                ..Default::default()
            }).insert_bundle(boid::BoidMindBundle{
                ..Default::default()
            })
            .with_children(|parent| {
                // the model
                parent
                    .spawn()
                    .insert(Name::new("model"))
                    .insert_bundle((
                        Transform::from_rotation(Quat::from_rotation_y(math::real::consts::PI)),
                        GlobalTransform::default(),
                    ))
                    .with_children(|parent| {
                        parent.spawn_scene(ball_fighter_model.clone());
                    });
                parent
                    .spawn()
                    .insert_bundle(craft::attire::AttireBundle {
                        profile: craft::attire::AttireProfile {
                            ..Default::default()
                        },
                        collider: ColliderBundle {
                            shape: ColliderShape::ball(4.).into(),
                            ..craft::attire::AttireBundle::default_collider_bundle()
                        },
                        ..Default::default()
                });
            })
            .id();
        commands
            .spawn()
            .insert_bundle({
                let mut cam = PerspectiveCameraBundle::default();
                cam.perspective_projection.far = 10_000.;
                cam
            })
            .insert_bundle(bevy_mod_picking::PickingCameraBundle::default())
            .insert(mind::player::CraftCamera {
                // target: Some(player_craft_id),
                ..mind::player::CraftCamera::default()
            });
        commands.insert_resource(mind::player::CurrentCraft(player_craft_id));
        /* let ball = commands
        .spawn()
        .insert_bundle(RigidBodyBundle {
            position: [0., 0., -70.].into(),
            body_type: RigidBodyType::Dynamic.into(),
            ..Default::default()
        })
        .insert_bundle(ColliderBundle {
            shape: ColliderShape::ball(1.).into(),
            ..Default::default()
        })
        .insert(ColliderDebugRender::default())
        .insert(ColliderPositionSync::Discrete)
        .id(); */
        /* commands.spawn().insert(JointBuilderComponent::new(
            SphericalJoint::new().local_anchor1([0., 0., -10.].into()),
            player_craft_id,
            ball,
        )); */
        /* commands.spawn().insert(JointBuilderComponent::new(
            PrismaticJoint::new(nalgebra::Unit::new_normalize((-TVec3::Z).into()))
                // .motor_position(20., 0.2, 0.1)
                .motor_velocity(10., 1.),
            player_craft_id,
            ball,
        )); */

        // spawn player weapon
        let wpn_id = commands
            .spawn()
            .insert_bundle(new_kinetic_cannon(player_craft_id))
            .insert_bundle(PbrBundle {
                mesh: meshes.add(shape::Cube { size: 1. }.into()),
                transform: Transform::from_translation(TVec3::Y * 0.)
                    .with_scale([1., 1., 4.].into()),
                material: materials.add(Color::WHITE.into()),
                ..Default::default()
            })
            .insert(Parent(player_craft_id))
            .id();
        commands.insert_resource(mind::player::CurrentWeapon(wpn_id));
    }
    let mut members = flock::FlockMembers::default();
    // spawn the ai craft
    for ii in -7..=7 {
        // for ii in 0..1 {
        members.push(commands
            .spawn()
            .insert_bundle(craft::CraftBundle {
                name: Name::new(format!("ai {ii}")),
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
                        shape: ColliderShape::ball(4.).into(),
                        mass_properties: ColliderMassProps::Density(
                            15_000. / (4. * math::real::consts::PI * 4. * 4.),
                        ).into(),
                        ..craft::attire::CollisionDamageEnabledColliderBundle::default_collider_bundle()
                    },
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert_bundle(boid::BoidMindBundle{
                directive:boid::BoidMindDirective::RunCircuit{param:boid::strategy::run_circuit::RunCircuit { initial_point }},
                ..Default::default()
            })
            .with_children(|parent| {
                let parent_entt = parent.parent_entity();
                parent
                    .spawn()
                    .insert(Name::new("model"))
                    .insert_bundle((
                        Transform::from_rotation(Quat::from_rotation_y(math::real::consts::PI)),
                        GlobalTransform::default(),
                    ))
                    .with_children(|parent| {
                        parent.spawn_scene(ball_fighter_model.clone());
                    });

                parent
                    .spawn()
                    .insert_bundle(craft::attire::AttireBundle {
                        profile: craft::attire::AttireProfile {
                            ..Default::default()
                        },
                        collider: ColliderBundle {
                            shape: ColliderShape::ball(4.).into(),
                            ..craft::attire::AttireBundle::default_collider_bundle()
                        },
                        ..Default::default()
                    });
                parent
                    .spawn()
                    .insert_bundle(new_kinetic_cannon(parent_entt))
                    .insert_bundle(PbrBundle {
                        mesh: meshes.add(shape::Cube { size: 1. }.into()),
                        transform: {
                            let mut t = Transform::from_translation(TVec3::Y * 0.);
                            t.scale = [1., 1., 4.].into();
                            t
                        },
                        material: materials.add(Color::WHITE.into()),
                        ..Default::default()
                    });
            }).id());
    }

    /* let flock_entt = commands.spawn().insert(Name::new("flock")).id();
    let formation = commands
        .spawn()
        .insert_bundle(flock::formation::FlockFormationBundle::new(
            flock::formation::FormationPattern::Sphere {
                center: flock::formation::FormationPivot::Anchor {
                    xform: Transform::from_translation([0., 0., -300.].into()),
                },
                radius: 150.,
            },
            flock::formation::SlottingStrategy::Simple,
            flock_entt,
        ))
        .id();
    commands
        .entity(flock_entt)
        .insert_bundle(flock::FlockMindBundle {
            members,
            directive: flock::FlockMindDirective::HoldPosition {
                pos: [0., 0., -300.].into(),
                formation,
            },
            ..Default::default()
        }); */
}

#[allow(unreachable_code)]
fn craft_state_display(
    mut egui_context: ResMut<EguiContext>,
    cur_craft: Res<mind::player::CurrentCraft>,
    craft_cameras: Query<&mind::player::CraftCamera>,
    mut crafts: Query<(
        &GlobalTransform,
        &craft::engine::LinearEngineState,
        &craft::engine::AngularEngineState,
        &mut craft::engine::LinearDriverPid,
        &mut craft::engine::AngularDriverPid,
    )>,
) {
    let (craft_xform, lin_state, ang_state, mut lin_pid, mut ang_pid) =
        crafts.get_mut(cur_craft.0).unwrap_or_log();
    let cam = craft_cameras.single();
    egui::Window::new("Status")
        .collapsible(true)
        .default_pos([1100., 0.])
        .show(egui_context.ctx_mut(), |ui| {
            ui.label(format!("position:      {:+03.1?}", craft_xform.translation));
            ui.label(format!("linear vel:    {:+03.1?}", lin_state.velocity));
            ui.label(format!("linear input:  {:+03.1?}", lin_state.input));
            ui.label(format!("linear flame:  {:+03.1?}", lin_state.flame));
            ui.label(format!("angular vel:   {:+03.1?}", ang_state.velocity));
            ui.label(format!("angular input: {:+03.1?}", ang_state.input));
            ui.label(format!("angular flame: {:+03.1?}", ang_state.flame));

            ui.label(format!("cam facing dir: {:+03.1?}", cam.facing_direction));
            ui.label(format!("craft forward: {:+03.1?}", craft_xform.forward()));

            return;
            ui.separator();
            ui.label("linear pid tune");
            {
                let mut proportional_gain = lin_pid.0.proportional_gain.x;
                ui.add(
                    egui::Slider::new(&mut proportional_gain, 0.0..=10_000.)
                        .clamp_to_range(false)
                        .text("p gain"),
                );
                lin_pid.0.proportional_gain = [proportional_gain; 3].into();
            }

            {
                let mut integral_gain = lin_pid.0.integrat_gain.x;
                ui.add(
                    egui::Slider::new(&mut integral_gain, 0.0..=1.)
                        .clamp_to_range(false)
                        .text("i gain"),
                );
                lin_pid.0.integrat_gain = [integral_gain; 3].into();
            }

            {
                let mut differntial_gain = lin_pid.0.differntial_gain.x;
                ui.add(
                    egui::Slider::new(&mut differntial_gain, 0.0..=1000.)
                        .clamp_to_range(false)
                        .text("d gain"),
                );
                lin_pid.0.differntial_gain = [differntial_gain; 3].into();
            }

            ui.separator();
            ui.label("angular pid tune");
            {
                let mut proportional_gain = ang_pid.0.proportional_gain.x;
                ui.add(
                    egui::Slider::new(&mut proportional_gain, 0.0..=10_000.)
                        .clamp_to_range(false)
                        .text("p gain"),
                );
                ang_pid.0.proportional_gain = [proportional_gain; 3].into();
            }

            {
                let mut integral_gain = ang_pid.0.integrat_gain.x;
                ui.add(
                    egui::Slider::new(&mut integral_gain, 0.0..=1.)
                        .clamp_to_range(false)
                        .text("i gain"),
                );
                ang_pid.0.integrat_gain = [integral_gain; 3].into();
            }

            {
                let mut differntial_gain = ang_pid.0.differntial_gain.x;
                ui.add(
                    egui::Slider::new(&mut differntial_gain, 0.0..=1000.)
                        .clamp_to_range(false)
                        .text("d gain"),
                );
                ang_pid.0.differntial_gain = [differntial_gain; 3].into();
            }
            //ui.label(format!("lnear pid: {:+03.1?}", lin_pid));
            //ui.label(format!("angular pid: {:+03.1?}", ang_pid));
        });
}

#[derive(Debug, Clone, Copy, Component)]
pub struct GameCamera;

#[derive(Debug, Clone, Default)]
pub struct CameraMovementSettings {
    linear_speed: TReal,
    angular_speed: TReal,
    shift_multiplier: TReal,
    linear_input: IVec3,
    angular_input: IVec3,
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

        cam_settings.linear_input = linear_input.clamp(-IVec3::ONE, IVec3::ONE);
        cam_settings.angular_input = angular_input.clamp(-IVec3::ONE, IVec3::ONE);
        cam_settings.shift_on = shift_on;
    }

    let mut linear_speed = cam_settings.linear_speed;

    if cam_settings.shift_on {
        linear_speed *= cam_settings.shift_multiplier
    }

    let delta_t = time.delta_seconds_f64() as TReal;
    let linear_vel = cam_settings.linear_input.as_vec3() * (linear_speed * delta_t);
    let angular_vel = cam_settings.angular_input.as_vec3() * (cam_settings.angular_speed * delta_t);

    // tracing::info!("linear_vel: {}, angular_vel: {}", linear_vel, angular_vel);

    let rotator = Quat::from_euler(EulerRot::YXZ, angular_vel.y, angular_vel.x, angular_vel.z);
    for mut camera_xform in cameras.iter_mut() {
        let cam_rotation = camera_xform.rotation;
        camera_xform.translation += cam_rotation * linear_vel;
        camera_xform.rotation *= rotator;
        // tracing::info!("resulting xform: {camera_xform:?}");
    }
}

#[test]
fn zmblo() {
    assert_eq!(Transform::identity(), Transform::identity().inverse());
}

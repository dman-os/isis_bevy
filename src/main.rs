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
    *,
};
use bevy_egui::*;
use bevy_rapier3d::prelude::*;

#[bevy_main]
fn main() -> Result<()> {
    // let subscriber = tracing_subscriber::fmt()
    //     .pretty()
    //     .compact()
    //     .with_timer(tracing_subscriber::fmt::time::uptime())
    //     // setup the env filter
    //     .with_env_filter(
    //         tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
    //             // if not found in env, just set a default at INFO level
    //             tracing_subscriber::EnvFilter::from_default_env()
    //                 .add_directive(tracing::Level::INFO.into())
    //         }),
    //     )
    //     // completes the builder.
    //     .finish();

    // tracing::subscriber::set_global_default(subscriber)
    //     .expect("setting default tracing subscriber failed");

    // tracing::info!("Hello, arrayer!");

    // #[cfg(feature = "dylink")]
    // tracing::info!("dylink enabled");

    App::build()
        .add_plugins(DefaultPlugins)
        .insert_resource(WindowDescriptor {
            title: "ISIS".to_string(),
            vsync: false,
            ..Default::default()
        })
        .add_plugin(EguiPlugin)
        .add_plugin(RapierRenderPlugin)
        .add_plugin(DiagnosticsPlugin)
        // .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(EntityCountDiagnosticsPlugin)
        .add_plugin(FrameTimeDiagnosticsPlugin)
        .add_plugin(GamePlugin)
        .run();

    Ok(())
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
            .insert_resource(RapierConfiguration {
                gravity: [0.0, 0.0, 0.0].into(),
                ..Default::default()
            })
            .add_startup_system(setup_fps_display.system())
            .add_system(craft_state_display.system())
            .add_system(text_update_system.system())
            .insert_resource(CameraMovementSettings {
                angular_speed: std::f32::consts::PI / 2.,
                linear_speed: 20.0,
                shift_multiplier: 4.0,
                ..Default::default()
            })
            .add_system(move_camera_system.system())
            .add_startup_system(setup_environment.system())
            .add_startup_system(setup_world.system());
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
                            font_size: 30.0,
                            color: Color::WHITE,
                        },
                    },
                    TextSection {
                        value: "".to_string(),
                        style: TextStyle {
                            font: asset_server.load("fonts/test_font.ttf"),
                            font_size: 30.0,
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

#[derive(Debug, Clone, Copy)]
pub struct GameCamera;

#[derive(Debug, Clone, Copy, Default)]
pub struct CameraMovementSettings {
    linear_speed: f32,
    angular_speed: f32,
    shift_multiplier: f32,
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

    let delta_t = time.delta_seconds_f64() as f32;
    let linear_vel = cam_settings.linear_input.as_f32() * (linear_speed * delta_t);
    let angular_vel = cam_settings.angular_input.as_f32() * (cam_settings.angular_speed * delta_t);

    // tracing::info!("linear_vel: {}, angular_vel: {}", linear_vel, angular_vel);

    let rotator = Quat::from_rotation_ypr(angular_vel.y, angular_vel.x, angular_vel.z);
    for mut camera_xform in cameras.iter_mut() {
        let cam_rotation = camera_xform.rotation;
        camera_xform.translation += cam_rotation * linear_vel;
        camera_xform.rotation *= rotator;
    }

    // tracing::info!("resulting xform: {:?}", camera_xform);
}

fn craft_state_display(
    egui_context: ResMut<EguiContext>,
    cur_craft: Res<CurrentCraft>,
    crafts: Query<(&Transform, &RigidBodyVelocity), With<Craft>>,
) {
    let (craft_xform, vel) = crafts.get(cur_craft.0).unwrap();
    egui::Window::new("Status").show(egui_context.ctx(), |ui| {
        ui.label(format!("position: {:?}", craft_xform.translation));
        ui.label(format!("linear vel: {}", vel.linvel));
        ui.label(format!("angular vel: {}", vel.angvel));
    });
}

fn setup_environment(
    mut commands: Commands,
    // mut meshes: ResMut<Assets<Mesh>>,
    // mut materials: ResMut<Assets<StandardMaterial>>,
    // asset_server: Res<AssetServer>,
) {
    // light
    commands.spawn_bundle(LightBundle {
        // transform: Transform::from_xyz(4.0, 8.0, 4.0),
        transform: Transform::from_xyz(0.0, 30.0, 0.),
        light: Light {
            range: 200.,
            intensity: 549.,
            ..Default::default()
        },
        ..Default::default()
    });

    // // camera
    // commands
    //     .spawn_bundle(PerspectiveCameraBundle {
    //         transform: Transform::from_xyz(-20.0, 25., 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    //         ..Default::default()
    //     })
    //     .insert(GameCamera);
}

pub struct CraftCamera;

pub struct Craft;
pub struct CurrentCraft(pub Entity);

fn setup_world(
    mut commands: Commands,
    // mut meshes: ResMut<Assets<Mesh>>,
    // mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // use bevy::render::mesh::shape;
    // const GROUN_PLANE_LENGTH: f32 = 128.;

    // // spawns a white plane at one unit below the orign
    // commands
    //     .spawn_bundle(PbrBundle {
    //         mesh: meshes.add(Mesh::from(shape::Plane {
    //             size: GROUN_PLANE_LENGTH,
    //         })),
    //         material: materials.add(Color::WHITE.into()),
    //         transform: Transform::from_translation(Vec3::Y / 2.0),
    //         ..Default::default()
    //     })
    //     .insert_bundle(ColliderBundle {
    //         shape: ColliderShape::cuboid(GROUN_PLANE_LENGTH, 0.1, GROUN_PLANE_LENGTH),
    //         ..Default::default()
    //     });

    // // cube
    // commands
    //     .spawn_bundle(PbrBundle {
    //         mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
    //         transform: Transform::from_translation(Vec3::Y * 10.0),
    //         material: materials.add(Color::BEIGE.into()),
    //         ..Default::default()
    //     })
    //     .insert_bundle(RigidBodyBundle {
    //         position: [0., 10., 0.].into(),
    //         ..Default::default()
    //     })
    //     .insert_bundle(ColliderBundle {
    //         shape: ColliderShape::cuboid(1., 1., 1.),
    //         ..Default::default()
    //     })
    //     .insert(RigidBodyPositionSync::Discrete);

    // Spawn the craft
    let current_craft_id = commands
        .spawn_bundle((
            Transform::from_xyz(0.0, 10.0, 00.0),
            GlobalTransform::identity(),
        ))
        .insert(Craft)
        .insert_bundle(RigidBodyBundle {
            position: [0., 10., 0.].into(),
            ..Default::default()
        })
        .insert(RigidBodyPositionSync::Discrete)
        .with_children(|parent| {
            // the model
            parent.spawn_scene(asset_server.load("models/ball_fighter.gltf#Scene0"));
            // the colliders
            parent.spawn_bundle(ColliderBundle {
                shape: ColliderShape::ball(4.),
                ..Default::default()
            });

            // parent
            //     .spawn_bundle((
            //         Transform::from_xyz(0.0, 0.0, 0.0),
            //         GlobalTransform::identity(),
            //     ))
            //     .with_children(|parent| {
            //         parent.spawn_bundle(ColliderBundle {
            //             shape: ColliderShape::ball(8.),
            //             ..Default::default()
            //         });
            //     });

            parent
                .spawn_bundle(PerspectiveCameraBundle {
                    transform: Transform::from_xyz(0.0, 7., -20.0).looking_at(Vec3::Z, Vec3::Y),
                    ..Default::default()
                })
                .insert(CraftCamera);
        })
        .id();

    commands.insert_resource(CurrentCraft(current_craft_id));
}

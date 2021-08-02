use deps::*;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::math::{Real, *};

pub struct ArmsPlugin;

impl Plugin for ArmsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(handle_activate_weapon_events.system())
            .add_system(cull_old_colliding_projectiles.system())
            .add_event::<ActivateWeaponEvent>()
            .add_event::<ProjectileIxnEvent>();
    }
}

pub struct ActivateWeaponEvent {
    pub weapon_id: Entity,
}

use crate::craft::attire::Damage;

pub struct ProjectileWeapon {
    pub proj_damage: Damage,
    pub proj_mesh: Handle<Mesh>,
    pub proj_mtr: Handle<StandardMaterial>,
    pub proj_velocity: Vector3,
    pub proj_shape: ColliderShape,
    pub proj_lifespan_secs: f64,
    pub proj_spawn_offset: Vector3,
}

//pub struct CraftArms(pub Children);
#[derive(Debug, Clone)]
pub struct Projectile {
    pub damage: Damage,
    pub source_wpn: Entity,
    pub emit_instant_secs: f64,
    pub lifespan_secs: f64,
}

fn handle_activate_weapon_events(
    //crafts: Query<&CraftArms>,
    mut commands: Commands,
    weapons: Query<(&ProjectileWeapon, &GlobalTransform)>,
    mut fire_events: EventReader<ActivateWeaponEvent>,
    //mut lines: ResMut<bevy_prototype_debug_lines::DebugLines>,
    time: Res<Time>,
) {
    for event in fire_events.iter() {
        match weapons.get(event.weapon_id) {
            Ok((proj_wpn, xform)) => {
                //lines.line(
                //xform.translation,
                //xform.translation + (xform.rotation * proj_wpn.proj_spawn_offset),
                //1.,
                //);
                commands
                    .spawn()
                    .insert(Projectile {
                        damage: proj_wpn.proj_damage,
                        lifespan_secs: proj_wpn.proj_lifespan_secs,
                        source_wpn: event.weapon_id,
                        emit_instant_secs: time.seconds_since_startup(),
                    })
                    .insert_bundle(PbrBundle {
                        mesh: proj_wpn.proj_mesh.clone(),
                        material: proj_wpn.proj_mtr.clone(),
                        ..Default::default()
                    })
                    .insert_bundle(RigidBodyBundle {
                        //body_type: RigidBodyType::KinematicVelocityBased,
                        position: RigidBodyPosition {
                            position: (xform.translation
                                + (xform.rotation * proj_wpn.proj_spawn_offset))
                                .into(),
                            ..Default::default()
                        },
                        velocity: RigidBodyVelocity {
                            linvel: <[Real; 3]>::from(xform.rotation * proj_wpn.proj_velocity)
                                .into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .insert(RigidBodyPositionSync::Discrete)
                    .insert_bundle(ColliderBundle {
                        shape: proj_wpn.proj_shape.clone(),
                        collider_type: ColliderType::Sensor,
                        ..Default::default()
                    });
            }
            Err(err) => {
                tracing::warn!(
                    "ActivateWeaponEvent for unrecognized wepon_id ({:?}): {:?}",
                    event.weapon_id,
                    err
                );
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProjectileIxnEvent {
    pub projectile: Projectile,
    pub collider: ColliderHandle,
}

fn cull_old_colliding_projectiles(
    mut commands: Commands,
    projectiles: Query<(Entity, &Projectile)>,
    narrow_phase: Res<NarrowPhase>,
    time: Res<Time>,
    mut ixn_events: EventWriter<ProjectileIxnEvent>,
) {
    for (entity, proj) in projectiles.iter() {
        let mut despawn = false;
        // if our projectile is intersecting with anything
        for (collider1, collider2, ixning) in narrow_phase.intersections_with(entity.handle()) {
            if ixning {
                ixn_events.send(ProjectileIxnEvent {
                    projectile: proj.clone(),
                    collider: if collider1 != entity.handle() {
                        collider1
                    } else {
                        collider2
                    },
                });
                despawn = true;
            }
        }
        // or if it's expired
        if despawn || (time.seconds_since_startup() - proj.emit_instant_secs) > proj.lifespan_secs {
            commands.entity(entity).despawn_recursive();
            tracing::trace!("projectile {:?} despawned", entity);
        }
    }
}

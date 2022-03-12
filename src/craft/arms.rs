use deps::*;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::craft::attire::*;
use crate::math::*;

pub struct ArmsPlugin;

impl Plugin for ArmsPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(handle_activate_weapon_events_projectile)
            .add_system(cull_old_colliding_projectiles)
            .add_event::<ActivateWeaponEvent>()
            .add_event::<ProjectileIxnEvent>();
    }
}
/// A generic bundle for craft strategies.
#[derive(Bundle)]
pub struct WeaponBundle<P>
where
    P: Component,
{
    pub param: P,
    pub tag: CraftWeapon,
    pub name: Name,
    pub activation_state: WeaponActivationState,
}

impl<P> WeaponBundle<P>
where
    P: Component,
{
    pub const DEFAULT_NAME: &'static str = "weapon";
    pub fn new(
        param: P,
        boid_entt: Entity,
        class: WeaponClass,
        activation_state: WeaponActivationState,
    ) -> Self {
        Self {
            param,
            tag: CraftWeapon::new(boid_entt, WeaponKind::of::<P>(), class),
            activation_state,
            // name: Self::DEFAULT_NAME.into(),
            name: class.into(),
        }
    }
}

pub type WeaponClass = &'static str;
pub type WeaponKind = std::any::TypeId;

/// This tags an entity as a steering routine
#[derive(Debug, Clone, Copy, Component)]
pub struct CraftWeapon {
    boid_entt: Entity,
    kind: WeaponKind,
    class: WeaponClass,
}

impl CraftWeapon {
    pub fn new(boid_entt: Entity, kind: WeaponKind, class: WeaponClass) -> Self {
        Self {
            boid_entt,
            kind,
            class,
        }
    }

    /// Get a reference to the craft weapon's craft entt.
    #[inline]
    pub fn boid_entt(&self) -> Entity {
        self.boid_entt
    }

    /// Get a reference to the craft weapon's kind.
    #[inline]
    pub fn kind(&self) -> WeaponKind {
        self.kind
    }

    /// Get a reference to the craft weapon's class.
    pub fn class(self) -> WeaponClass {
        self.class
    }
}

pub struct ActivateWeaponEvent {
    pub weapon_id: Entity,
}

#[derive(Debug, Clone, Component)]
pub enum WeaponActivationState {
    Discrete {
        firing_rate: f64,
        last_firing_time: f64,
    },
}

impl WeaponActivationState {
    pub fn new_discrete(firing_rate: f64) -> Self {
        Self::Discrete {
            firing_rate,
            last_firing_time: 0.,
        }
    }
    pub fn can_activate(&self, time: &Time) -> bool {
        match self {
            WeaponActivationState::Discrete {
                firing_rate: weapon_firing_rate,
                last_firing_time,
            } => (time.seconds_since_startup() - last_firing_time) > (1. / weapon_firing_rate),
        }
    }
}

#[derive(Component)]
pub struct ProjectileWeapon {
    pub proj_damage: Damage,
    pub proj_mesh: Handle<Mesh>,
    pub proj_mtr: Handle<StandardMaterial>,
    pub proj_velocity: TVec3, // TODO: replace with speed
    pub proj_shape: ColliderShape,
    pub proj_mass: ColliderMassProps,
    pub proj_lifespan_secs: f64,
    pub proj_spawn_offset: TVec3,
}

#[derive(Debug, Clone, Component)]
pub struct Projectile {
    pub damage: Damage,
    pub source_wpn: Entity,
    pub emit_instant_secs: f64,
    pub lifespan_secs: f64,
}

fn handle_activate_weapon_events_projectile(
    //crafts: Query<&CraftArms>,
    mut commands: Commands,
    mut weapons: Query<(
        &ProjectileWeapon,
        &mut WeaponActivationState,
        &GlobalTransform,
    )>,
    mut fire_events: EventReader<ActivateWeaponEvent>,
    //mut lines: ResMut<bevy_prototype_debug_lines::DebugLines>,
    time: Res<Time>,
) {
    for event in fire_events.iter() {
        match weapons.get_mut(event.weapon_id) {
            Ok((proj_wpn, mut firing_state, xform)) => {
                /* tracing::info!(
                    "\n{:?}\n{:?}",
                    xform.forward(),
                    (xform.rotation * proj_wpn.proj_velocity).normalize()
                ); */
                if !firing_state.can_activate(&time) {
                    continue;
                }
                match firing_state.as_mut() {
                    WeaponActivationState::Discrete {
                        last_firing_time, ..
                    } => *last_firing_time = time.seconds_since_startup(),
                }
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
                        ccd: RigidBodyCcd {
                            ccd_enabled: true,
                            ccd_active: true,
                            ccd_thickness: proj_wpn.proj_shape.ccd_thickness(),
                            ccd_max_dist: proj_wpn.proj_shape.ccd_thickness() * 0.5,
                            // ..Default::default()
                        }
                        .into(),
                        position: RigidBodyPosition {
                            position: (
                                xform.translation + (xform.rotation * proj_wpn.proj_spawn_offset),
                                xform.rotation,
                            )
                                .into(),
                            ..Default::default()
                        }
                        .into(),
                        velocity: RigidBodyVelocity {
                            linvel: <[TReal; 3]>::from(xform.rotation * proj_wpn.proj_velocity)
                                .into(),
                            ..Default::default()
                        }
                        .into(),
                        ..Default::default()
                    })
                    .insert(RigidBodyPositionSync::Interpolated { prev_pos: None })
                    .insert_bundle(ColliderBundle {
                        shape: ColliderShapeComponent(proj_wpn.proj_shape.clone()),
                        collider_type: ColliderType::Sensor.into(),
                        // TODO: massive projectiles
                        // mass_properties: proj_wpn.proj_mass.clone(),
                        flags: ColliderFlags {
                            active_events: ActiveEvents::INTERSECTION_EVENTS,
                            collision_groups: *PROJECTILE_COLLIDER_IGROUP,
                            ..Default::default()
                        }
                        .into(),
                        ..Default::default()
                    });
            }
            Err(err) => {
                tracing::warn!(
                    "ActivateWeaponEvent for unrecognized wepon_id ({:?}): {err:?}",
                    event.weapon_id,
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

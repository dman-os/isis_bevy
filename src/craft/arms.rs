use deps::*;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_rapier3d::rapier::prelude::SharedShape;

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
    pub proj_shape: SharedShape,
    pub proj_mass: ColliderMassProperties,
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
                let xform = xform.compute_transform();
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
                    .insert(Name::new("projectile"))
                    .insert(Projectile {
                        damage: proj_wpn.proj_damage,
                        lifespan_secs: proj_wpn.proj_lifespan_secs,
                        source_wpn: event.weapon_id,
                        emit_instant_secs: time.seconds_since_startup(),
                    })
                    .insert_bundle(PbrBundle {
                        mesh: proj_wpn.proj_mesh.clone(),
                        material: proj_wpn.proj_mtr.clone(),
                        transform: Transform::from_translation(
                            xform.translation + (xform.rotation * proj_wpn.proj_spawn_offset),
                        )
                        .with_rotation(xform.rotation),
                        ..default()
                    })
                    .insert(RigidBody::Dynamic)
                    .insert(
                        // TODO: inherit craft velocity too
                        Velocity {
                            linvel: (xform.rotation * proj_wpn.proj_velocity),
                            ..default()
                        },
                    )
                    .insert(Ccd::enabled())
                    .insert(TransformInterpolation::default())
                    /* ccd_thickness: proj_wpn.proj_shape.ccd_thickness(),
                    ccd_max_dist: proj_wpn.proj_shape.ccd_thickness() * 0.5, */
                    .insert(Collider::from(proj_wpn.proj_shape.clone()))
                    .insert(Sensor)
                    .insert(ActiveEvents::COLLISION_EVENTS)
                    // TODO: massive projectiles
                    // .insert(proj_wpn.proj_mass);
                    .insert(*PROJECTILE_COLLIDER_IGROUP);
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
    pub collider: Entity,
}

fn cull_old_colliding_projectiles(
    mut commands: Commands,
    projectiles: Query<(Entity, &Projectile)>,
    // FIXME: consider using RapierCtx
    mut collision_events: EventReader<CollisionEvent>,
    time: Res<Time>,
    mut ixn_events: EventWriter<ProjectileIxnEvent>,
    mut despawn_set: Local<bevy::utils::HashSet<Entity>>,
) {
    for collision_event in collision_events.iter() {
        if let CollisionEvent::Started(coll1, coll2, _) = *collision_event {
            // if flags == CollisionEventFlags::SENSOR {}

            // if any of our collider is a projectile
            if let Ok((proj_coll, proj)) =
                projectiles.get(coll1).or_else(|_| projectiles.get(coll2))
            {
                ixn_events.send(ProjectileIxnEvent {
                    projectile: proj.clone(),
                    collider: if proj_coll == coll1 { coll2 } else { coll1 },
                });
                despawn_set.insert(proj_coll);
            }
        };
    }
    for (entt, proj) in projectiles.iter() {
        // test expired items
        if (time.seconds_since_startup() - proj.emit_instant_secs) > proj.lifespan_secs {
            despawn_set.insert(entt);
        }
    }
    for entt in despawn_set.drain() {
        tracing::trace!("projectile {:?} despawned", entt);
        commands.entity(entt).despawn_recursive();
    }
}

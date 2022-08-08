use deps::*;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_rapier3d::rapier::prelude::SharedShape;
use bitflags::bitflags;
use once_cell::sync::Lazy;

use crate::math::*;

pub struct AttirePlugin;
impl Plugin for AttirePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(handle_collision_damage_events)
            .add_system(handle_projectile_ixn_events)
            .add_system(log_damage_events)
            .add_event::<CollisionDamageEvent>()
            .add_event::<ProjectileDamageEvent>();
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DamageType {
    Beam,
    Collision,
    Explosion,
    Kinetic,
    Plasma,
}

impl Default for DamageType {
    fn default() -> Self {
        Self::Kinetic
    }
}

#[derive(Debug, Clone, Copy, Component)]
pub struct Damage {
    pub value: TReal,
    pub damage_type: DamageType,
}

#[derive(Debug, Clone, Copy)]
pub enum AttireType {
    Hull,
    Armour,
    Shield,
}
impl Default for AttireType {
    fn default() -> Self {
        Self::Hull
    }
}

/// A health bar for some craft component.
#[derive(Debug, Clone, Component)]
pub struct Attire {
    pub remaining_integrity: f32,

    //pub recovery_rate: Real,
    pub attire_type: AttireType,
    pub factory_integrity: f32,
    pub damage_multiplier: SVec<[f32; 6]>,
}

impl Attire {
    /// This applies damage to the attire and returns any damage that's left over if it's
    /// destroyed
    pub fn damage(&mut self, damage: Damage) -> Option<Damage> {
        let multiplier = self.damage_multiplier[damage.damage_type as usize];
        let true_damage = damage.value * multiplier;

        let new_integrity = self.remaining_integrity - true_damage;

        if new_integrity >= 0. {
            self.remaining_integrity = new_integrity;
            None
        } else {
            self.remaining_integrity = 0.;
            let remaining_damage = true_damage - self.remaining_integrity;
            let remaining_damage = remaining_damage / multiplier;
            Some(Damage {
                value: remaining_damage,
                damage_type: damage.damage_type,
            })
        }
    }
}

/// Mostly for UX purposes.
#[derive(Debug, Clone, Copy)]
pub enum AttireCoverage {
    Omni,
    Port,
    Bow,
    StarBoard,
    Stern,
}

impl Default for AttireCoverage {
    fn default() -> Self {
        Self::Omni
    }
}

/// A collider and a health bar(s) for location based damage to crafts.
#[derive(Debug, Clone, Component)]
pub struct AttireProfile {
    pub coverage: AttireCoverage,
    pub members: SVec<[Attire; 1]>,
}

impl AttireProfile {
    pub fn damage(&mut self, damage: Damage) -> Option<Damage> {
        let mut remaining_damage = Some(damage);
        for attire in self.members.iter_mut() {
            remaining_damage = attire.damage(remaining_damage.unwrap_or_log());
            if remaining_damage.is_none() {
                break;
            }
        }
        remaining_damage
    }
}

impl Default for AttireProfile {
    fn default() -> Self {
        AttireProfile {
            coverage: AttireCoverage::Omni,
            members: smallvec::smallvec![Attire {
                attire_type: AttireType::Hull,
                factory_integrity: 1_000.,
                remaining_integrity: 1_000.,
                damage_multiplier: smallvec::smallvec![1.0; 6],
            }],
        }
    }
}

bitflags! {
    pub struct ColliderGroups: u32 {
        const SOLID = 1 << 1;
        const CRAFT_SOLID = 1 << 2;
        const ATTIRE = 1 << 3;
        const PROJECTILE = 1 << 4;
        const SENSOR = 1 << 5;
    }
}

pub static CRAFT_COLLIDER_IGROUP: Lazy<CollisionGroups> = Lazy::new(|| {
    CollisionGroups::new(
        (ColliderGroups::CRAFT_SOLID).bits(),
        (ColliderGroups::SOLID
            | ColliderGroups::PROJECTILE
            | ColliderGroups::SENSOR
            | ColliderGroups::CRAFT_SOLID)
            .bits(),
    )
});
pub static ATTIRE_COLLIDER_IGROUP: Lazy<CollisionGroups> = Lazy::new(|| {
    CollisionGroups::new(
        ColliderGroups::ATTIRE.bits(),
        (ColliderGroups::PROJECTILE).bits(),
    )
});
pub static OBSTACLE_COLLIDER_IGROUP: Lazy<CollisionGroups> = Lazy::new(|| {
    CollisionGroups::new(
        ColliderGroups::SOLID.bits(),
        (ColliderGroups::SOLID | ColliderGroups::PROJECTILE | ColliderGroups::CRAFT_SOLID).bits(),
    )
});
pub static PROJECTILE_COLLIDER_IGROUP: Lazy<CollisionGroups> = Lazy::new(|| {
    CollisionGroups::new(
        (ColliderGroups::PROJECTILE).bits(),
        (ColliderGroups::ATTIRE | ColliderGroups::SOLID).bits(),
    )
});
pub static SENSOR_COLLIDER_IGROUP: Lazy<CollisionGroups> = Lazy::new(|| {
    CollisionGroups::new(
        (ColliderGroups::SENSOR).bits(),
        (ColliderGroups::PROJECTILE | ColliderGroups::SOLID | ColliderGroups::CRAFT_SOLID).bits(),
    )
});

#[derive(Bundle)]
pub struct AttireBundle {
    pub name: Name,
    pub profile: AttireProfile,
    pub collider: Collider,
    pub sensor: Sensor,
    pub collision_group: CollisionGroups,
    pub active_events: ActiveEvents,
}

impl AttireBundle {
    pub const DEFAULT_NAME: &'static str = "attire";
}

impl Default for AttireBundle {
    fn default() -> Self {
        Self {
            profile: AttireProfile::default(),
            collider: default(),
            name: Self::DEFAULT_NAME.into(),
            sensor: Sensor,
            collision_group: *ATTIRE_COLLIDER_IGROUP,
            active_events: ActiveEvents::COLLISION_EVENTS, // TODO: intersection event
        }
    }
}

/// Tags a rigidbody that's able to receive collision damage. This
/// requires a [`CollisionDamageEnabledCollider`] attached to function.
#[derive(Component)]
pub struct CollisionDamageEnabledRb;

/// Tags a collider that's able to detect collision damage. Note, this's
/// separate from the sensor collider attached to AttireProfiles.
#[derive(Component)]
pub struct CollisionDamageEnabledCollider;

#[derive(Bundle)]
pub struct CollisionDamageEnabledColliderBundle {
    pub collider: Collider,
    pub mass_props: ColliderMassProperties,
    pub collidsion_group: CollisionGroups,
    pub active_events: ActiveEvents,
    // pub threshold: ContactForceEventThreshold,
    // pub better_listener: BetterContactListener,
    pub tag: CollisionDamageEnabledCollider,
}

impl CollisionDamageEnabledColliderBundle {
    // pub const DEFAULT_FORCE_EVENT_THRESHOLD: f32 = 0.1;
}

impl Default for CollisionDamageEnabledColliderBundle {
    fn default() -> Self {
        Self {
            // better_listener: BetterContactListener,
            tag: CollisionDamageEnabledCollider,
            collider: default(),
            mass_props: default(),
            collidsion_group: *CRAFT_COLLIDER_IGROUP,
            active_events: ActiveEvents::COLLISION_EVENTS,
            // threshold: ContactForceEventThreshold(Self::DEFAULT_FORCE_EVENT_THRESHOLD),
        }
    }
}

/// Not really being consumed by anyone
/// but I'm going to leave it here.
#[derive(Clone)]
pub struct CollisionDamageEvent {
    rb_entt: Entity,
    attire_entt: Entity,
    damage: Damage,
    // contact_event: BetterContactEvent,
    is_entt_1: bool,
    /// The shape of the attire that included the deepest contact point
    /// so that had this attire ended up being selected for taking damage.
    #[allow(dead_code)]
    selection_shape: SharedShape,
    /// The position of the `selection_shape` during selection.
    selection_position: (Vec3, Quat),
}

impl std::fmt::Debug for CollisionDamageEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CollisionDamageEvent")
            .field("damage", &self.damage)
            .field("rb_entt", &self.rb_entt)
            .field("attire_entt", &self.attire_entt)
            // .field("contact_Event", &self.contact_event)
            .field("is_entt_1", &self.is_entt_1)
            .field("selection_position", &self.selection_position)
            .finish_non_exhaustive()
    }
}

/// Consumes [`BetterContactEvent`]s and damages [`AttireProfile`]s when
/// the object colliding has one attached.
pub(super) fn handle_collision_damage_events(
    listeners: Query<Entity, With<CollisionDamageEnabledCollider>>,
    time: Res<Time>,
    crafts: Query<(&crate::Colliders, &GlobalTransform), With<CollisionDamageEnabledRb>>,
    mut attires: Query<(&mut AttireProfile, &Collider, &GlobalTransform)>,
    mut cd_events: EventWriter<CollisionDamageEvent>,
    mut generated_events: Local<Vec<CollisionDamageEvent>>,
    rapier: Res<RapierContext>,
) {
    for entity in listeners.iter() {
        for contact_pair in rapier
            .contacts_with(entity)
            .filter(|c| c.has_any_active_contacts())
        {
            /* let other = if contact_pair.collider1() == entity {
                contact_pair.collider2()
            } else {
                contact_pair.collider1()
            }; */
            // find the deepest contact
            let (manifold, contact) = contact_pair.find_deepest_contact().unwrap_or_log();
            let damage = {
                // calculate the force from the impulse
                // J = F Δt
                // F = J / Δt
                let value = contact.impulse().abs();

                // ignore zero damage values
                if value <= TReal::EPSILON {
                    continue;
                }
                let value = value / time.delta_seconds();
                Damage {
                    value,
                    damage_type: DamageType::Collision,
                }
            };

            let mut rigd_body_involved = false;
            for val in [
                manifold
                    .rigid_body1()
                    .map(|rb| crafts.get(rb).map(|(colls, gx)| (rb, true, colls, gx))),
                manifold
                    .rigid_body2()
                    .map(|rb| crafts.get(rb).map(|(colls, gx)| (rb, false, colls, gx))),
            ] {
                let (rb_entt, is_entt_1, colls, g_xform) = match val {
                    Some(Ok(val)) => val,
                    _ => continue,
                };
                rigd_body_involved = true;
                let point = {
                    let point = if is_entt_1 {
                        contact.local_p1()
                    } else {
                        contact.local_p2()
                    };

                    g_xform.mul_vec3(point)
                };

                let mut closest_attire = None;
                // let body = rapier.entity2body().get(&rb_entt).and_then(|h| rapier.bodies.get(*h)).unwrap_or_log();
                // for all the rigid body's colliders
                for coll_entt in colls.set.iter().cloned() {
                    // let coll_entt = rapier.collider_entity(*coll_entt).unwrap_or_log();
                    // FIXME: this seems expensive
                    //
                    // if the collider belongs to an attire
                    if let Ok((mut attire, coll, attire_g_xform)) = attires.get_mut(coll_entt) {
                        let xform = attire_g_xform.compute_transform();
                        let dist =
                            coll.distance_to_point(xform.translation, xform.rotation, point, true);
                        // if the collider contains the point
                        if dist < 0.1 {
                            attire.damage(damage);

                            // generate the event to let others know it was damaged
                            generated_events.push(CollisionDamageEvent {
                                damage,
                                rb_entt,
                                attire_entt: coll_entt,
                                // contact_event: event.clone(),
                                selection_shape: coll.raw.clone(),
                                selection_position: (xform.translation, xform.rotation),
                                is_entt_1,
                            });

                            // consider the event hadnled once we've damaged an attire
                            return;
                        } else {
                            closest_attire = if let Some((other, other_dist)) = closest_attire {
                                if dist < other_dist {
                                    Some((coll_entt, dist))
                                } else {
                                    Some((other, other_dist))
                                }
                            } else {
                                Some((coll_entt, dist))
                            }
                        }
                    }
                }
                if let Some((attire_entt, dist)) = closest_attire {
                    // FIXME: this is being emitted to frequently at eye raising distances
                    tracing::debug!(
                        "CollisonDamageEnabledRb collided but no attires covered deepest contact point, damaging closest attire with at diastance {dist:?}",
                    );
                    let (mut attire, coll, attire_g_xform) =
                        attires.get_mut(attire_entt).unwrap_or_log();
                    let xform = attire_g_xform.compute_transform();
                    attire.damage(damage);
                    // generate the event to let others know it was damaged
                    generated_events.push(CollisionDamageEvent {
                        damage,
                        rb_entt,
                        attire_entt,
                        // contact_event: event.clone(),
                        selection_shape: coll.raw.clone(),
                        selection_position: (xform.translation, xform.rotation),
                        is_entt_1,
                    });
                } else {
                    tracing::warn!("CollisonDamageEnabledRb registered but no attire found");
                }
            }
            // if there was a rigidbody involved in the contact

            if !rigd_body_involved {
                tracing::warn!(
                    coll1 = ?contact_pair.collider1(),
                    coll2 = ?contact_pair.collider2(),
                    "contact event without rigidbody"
                );
            }
        }
    }

    // FIXME: premature optimization
    // FIXME: is this even an optimization?
    cd_events.send_batch(generated_events.drain(..));
}

use crate::craft::arms::ProjectileIxnEvent;

pub struct ProjectileDamageEvent {
    pub ixn_event: ProjectileIxnEvent,
    pub attire_entt: Entity,
}
/// Consumes [`ProjectileIxnEvent`]s and damages [`AttireProfile`]s when
/// the object intersecting has one attached.
fn handle_projectile_ixn_events(
    // mut commands: Commands,
    rapier: Res<RapierContext>,
    mut attires: Query<(Entity, &mut AttireProfile)>,
    mut proj_ixn_events: EventReader<ProjectileIxnEvent>,
    mut pd_events: EventWriter<ProjectileDamageEvent>,
    names: Query<Option<&Name>>,
) {
    for event in proj_ixn_events.iter() {
        if let Ok((attire_entt, mut attire)) = attires.get_mut(event.collider) {
            let parent = rapier.collider_parent(attire_entt).unwrap_or_log();
            if attire.damage(event.projectile.damage).is_some() {
                if let Some(name) = names.get(parent).unwrap_or_log() {
                    tracing::info!("Craft {} destroyed by Projectile damage", name.as_str());
                } else {
                    tracing::info!("Craft {parent:?} destroyed by Projectile damage",);
                }
                // reset health
                for member in attire.members.iter_mut() {
                    member.remaining_integrity = member.factory_integrity;
                }
                // commands.entity(parent.handle.entity()).despawn_recursive();
            }
            // generate the event to let others know it was damaged
            pd_events.send(ProjectileDamageEvent {
                ixn_event: event.clone(),
                attire_entt,
            });
        }
    }
}

fn log_damage_events(
    mut coll_dmg_events: EventReader<CollisionDamageEvent>,
    mut proj_dmg_events: EventReader<ProjectileDamageEvent>,
    names: Query<Option<&Name>>,
) {
    for event in coll_dmg_events.iter() {
        if let Some(name) = names.get(event.rb_entt).unwrap_or_log() {
            tracing::info!("Collision {:?} | Craft: {:?}", event.damage, name.as_str());
        } else {
            tracing::info!("Collision {:?} | Craft: {:?}", event.damage, event.rb_entt);
        }
    }
    for event in proj_dmg_events.iter() {
        tracing::info!(
            "Projectile {:?} | Attire: {:?}",
            event.ixn_event.projectile.damage,
            event.attire_entt
        );
    }
}

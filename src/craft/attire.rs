use std::sync::Arc;

use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};
use bevy_rapier3d::prelude::*;
use bitflags::bitflags;
use deps::bevy::utils::HashMap;
use once_cell::sync::Lazy;

use crate::math::*;

pub struct AttirePlugin;
impl Plugin for AttirePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(generate_better_contact_events.system())
            .add_system(handle_collision_damage_events.system())
            .add_system(handle_collider_ixn_events.system())
            .add_system(log_damage_events.system())
            .add_event::<BetterContactEvent>()
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

#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone)]
pub struct Attire {
    pub remaining_integrity: f32,

    //pub recovery_rate: Real,
    pub attire_type: AttireType,
    pub factory_integrity: f32,
    pub damage_multiplier: smallvec::SmallVec<[f32; 6]>,
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
#[derive(Debug, Clone)]
pub struct AttireProfile {
    pub coverage: AttireCoverage,
    pub members: smallvec::SmallVec<[Attire; 1]>,
}

impl AttireProfile {
    pub fn damage(&mut self, damage: Damage) -> Option<Damage> {
        let mut remaining_damage = Some(damage);
        for attire in self.members.iter_mut() {
            remaining_damage = attire.damage(remaining_damage.unwrap());
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
        const ATTIRE = 1 << 2;
        const PROJECTILE = 1 << 3;
    }
}

pub const CRAFT_COLLIDER_IGROUP: Lazy<InteractionGroups> = Lazy::new(|| {
    InteractionGroups::new(
        ColliderGroups::SOLID.bits(),
        (ColliderGroups::SOLID | ColliderGroups::PROJECTILE).bits(),
    )
});
pub const ATTIRE_COLLIDER_IGROUP: Lazy<InteractionGroups> = Lazy::new(|| {
    InteractionGroups::new(
        ColliderGroups::ATTIRE.bits(),
        (ColliderGroups::PROJECTILE).bits(),
    )
});
pub const OBSTACLE_COLLIDER_IGROUP: Lazy<InteractionGroups> = Lazy::new(|| {
    InteractionGroups::new(
        ColliderGroups::SOLID.bits(),
        (ColliderGroups::SOLID | ColliderGroups::PROJECTILE).bits(),
    )
});
pub const PROJECTILE_COLLIDER_IGROUP: Lazy<InteractionGroups> = Lazy::new(|| {
    InteractionGroups::new(
        (ColliderGroups::PROJECTILE).bits(),
        (ColliderGroups::ATTIRE | ColliderGroups::SOLID).bits(),
    )
});

#[derive(Bundle)]
pub struct AttireBundle {
    pub profile: AttireProfile,
    #[bundle]
    pub collider: ColliderBundle,
}

impl AttireBundle {
    pub fn default_collider_bundle() -> ColliderBundle {
        ColliderBundle {
            collider_type: ColliderType::Sensor,
            flags: ColliderFlags {
                active_events: ActiveEvents::INTERSECTION_EVENTS,
                collision_groups: *ATTIRE_COLLIDER_IGROUP,
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

impl Default for AttireBundle {
    fn default() -> Self {
        Self {
            profile: AttireProfile::default(),
            collider: Self::default_collider_bundle(),
        }
    }
}

/// Tags a rigidbody that's able to receive collision damage. This
/// requires a [`CollisionDamageEnabledCollider`] attached to function.
pub struct CollisionDamageEnabledRb;

///// Tags a collider that's able to detect collision damage. Note, this's
///// separate from the sensor collider attached to AttireProfiles.
//pub struct CollisionDamageEnabledCollider;

#[derive(Bundle)]
pub struct CollisionDamageEnabledColliderBundle {
    #[bundle]
    pub collider: ColliderBundle,
    pub better_listener: BetterContactListener,
    //pub tag: CollisionDamageEnabledCollider,
}

impl CollisionDamageEnabledColliderBundle {
    pub fn default_collider_bundle() -> ColliderBundle {
        ColliderBundle {
            flags: ColliderFlags {
                active_events: ActiveEvents::CONTACT_EVENTS,
                collision_groups: *CRAFT_COLLIDER_IGROUP,
                ..Default::default()
            },
            ..Default::default()
        }
    }
}
impl Default for CollisionDamageEnabledColliderBundle {
    fn default() -> Self {
        Self {
            better_listener: BetterContactListener,
            //tag: CollisionDamageEnabledCollider,
            collider: Self::default_collider_bundle(),
        }
    }
}

/// Tags a collider so that more detailed contact events are
/// generated for it.
pub struct BetterContactListener;

/// A more detailed contact event that has data from the [`NarrowPhase`] graph.
#[derive(Clone)]
pub struct BetterContactEvent {
    id: (Entity, Entity),
    contact_pair: Arc<ContactPair>,
}

impl std::fmt::Debug for BetterContactEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BetterContactEvent")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

/// Not really being consumed by anyone
/// but I'm going to leave it here.
#[derive(Clone)]
pub struct CollisionDamageEvent {
    rb_entt: Entity,
    attire_entt: Entity,
    damage: Damage,
    contact_event: BetterContactEvent,
    is_entt_1: bool,
    /// The shape of the attire that included the deepest contact point
    /// so that had this attire ended up being selected for taking damage.
    selection_shape: ColliderShape,
    /// The position of the `selection_shape` during selection.
    selection_position: ColliderPosition,
}
impl std::fmt::Debug for CollisionDamageEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CollisionDamageEvent")
            .field("damage", &self.damage)
            .field("rb_entt", &self.rb_entt)
            .field("attire_entt", &self.attire_entt)
            .field("contact_Event", &self.contact_event)
            .field("is_entt_1", &self.is_entt_1)
            .field("selection_position", &self.selection_position)
            .finish_non_exhaustive()
    }
}

//#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, SystemLabel)]
//pub enum AttireSystems {
//GenerateBetterContactEvents,
//}

/// Generates [`BetterContactEvent`]s for registered listeners using the [`NarrowPhase`] graph..
pub(super) fn generate_better_contact_events(
    listeners: Query<Entity, With<BetterContactListener>>,
    mut bc_events: EventWriter<BetterContactEvent>,
    narrow_phase: Res<NarrowPhase>,
    mut contact_pair_store: Local<HashMap<(Entity, Entity), Arc<ContactPair>>>,
) {
    for entity in listeners.iter() {
        for contact_pair in narrow_phase
            .contacts_with(entity.handle())
            .filter(|c| c.has_any_active_contact)
        {
            let key = (
                contact_pair.collider1.entity(),
                contact_pair.collider2.entity(),
            );
            contact_pair_store
                .entry(key)
                .or_insert_with(|| Arc::new(contact_pair.clone()));
        }
    }
    bc_events.send_batch(
        contact_pair_store
            .drain()
            .map(|(id, contact_pair)| BetterContactEvent { id, contact_pair }),
    );
}

/// Consumes [`BetterContactEvent`]s and damages [`AttireProfile`]s when
/// the object colliding has one attached.
pub(super) fn handle_collision_damage_events(
    time: Res<Time>,
    crafts: Query<(Entity, &RigidBodyColliders, &GlobalTransform), With<CollisionDamageEnabledRb>>,
    mut attires: Query<(&mut AttireProfile, &ColliderShape, &ColliderPosition)>,
    mut contact_events: EventReader<BetterContactEvent>,
    mut cd_events: EventWriter<CollisionDamageEvent>,
    mut generated_events: Local<Vec<CollisionDamageEvent>>,
) {
    for event in contact_events.iter() {
        let (manifold, contact) = event.contact_pair.find_deepest_contact().unwrap();
        let damage = {
            // calculate the force from the impulse
            // J = F Δt
            // F = J / Δt
            let value = contact.data.impulse.abs();

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

        // if there was a rigidbody involved in the contact
        if let Some(rb_handle) = manifold.data.rigid_body1 {
            // if it's collision enabled
            if let Ok(set) = crafts.get(rb_handle.entity()) {
                inner(
                    true,
                    event,
                    set,
                    &mut attires,
                    &mut generated_events,
                    contact,
                    damage,
                );
            }
        } else {
            tracing::warn!("contact event without rigidbody");
        }

        if let Some(rb_handle) = manifold.data.rigid_body2 {
            if let Ok(set) = crafts.get(rb_handle.entity()) {
                inner(
                    false,
                    event,
                    set,
                    &mut attires,
                    &mut generated_events,
                    contact,
                    damage,
                );
            }
        } else {
            tracing::warn!("contact event without rigidbody");
        }
    }

    // FIXME: premature optimization
    // FIXME: is this even an optimization?
    cd_events.send_batch(generated_events.drain(0..));

    #[inline]
    fn inner(
        is_entt_1: bool,
        event: &BetterContactEvent,
        components: (Entity, &RigidBodyColliders, &GlobalTransform),
        attires: &mut Query<(&mut AttireProfile, &ColliderShape, &ColliderPosition)>,
        generated_events: &mut Vec<CollisionDamageEvent>,
        contact: &TrackedContact<ContactData>,
        damage: Damage,
    ) {
        // FIXME: this bugs out in instances where none of the
        // attires include the deepest point
        let point = {
            let point = if is_entt_1 {
                contact.local_p1
            } else {
                contact.local_p2
            };
            let point = components.2.mul_vec3(point.into());
            point.into()
        };

        let mut closest_attire = None;
        // for all the rigid body's colliders
        for collider in components.1 .0.iter() {
            // FIXME: this seems expensive
            //
            // if the collider belongs to an attire
            if let Ok((mut attire, coll_shape, coll_pos)) = attires.get_mut(collider.entity()) {
                // if the collider contains the point
                let dist = coll_shape.distance_to_point(coll_pos, &point, true);
                if dist < 0.1 {
                    attire.damage(damage);

                    // generate the event to let others know it was damaged
                    generated_events.push(CollisionDamageEvent {
                        damage,
                        rb_entt: components.0,
                        attire_entt: collider.entity(),
                        contact_event: event.clone(),
                        selection_shape: coll_shape.clone(),
                        selection_position: *coll_pos,
                        is_entt_1,
                    });

                    // consider the event hadnled once we've damaged an attire
                    return;
                } else {
                    closest_attire = if let Some((other, other_dist)) = closest_attire {
                        if dist < other_dist {
                            Some((collider.entity(), dist))
                        } else {
                            Some((other, other_dist))
                        }
                    } else {
                        Some((collider.entity(), dist))
                    }
                }
            }
        }
        if let Some((attire_entt, dist)) = closest_attire {
            tracing::warn!(
                "CollisonDamageEnabledRb collided but no attires covered deepest contact point, damaging closest attire ({:?}) with dist_sqr {:?}",
                attire_entt, dist
            );
            let (mut attire, coll_shape, coll_pos) = attires.get_mut(attire_entt).unwrap();
            attire.damage(damage);
            // generate the event to let others know it was damaged
            generated_events.push(CollisionDamageEvent {
                damage,
                rb_entt: components.0,
                attire_entt,
                contact_event: event.clone(),
                selection_shape: coll_shape.clone(),
                selection_position: *coll_pos,
                is_entt_1,
            });
        } else {
            tracing::warn!("CollisonDamageEnabledRb registered but no attire found");
        }
    }
}

use crate::craft::arms::ProjectileIxnEvent;

pub struct ProjectileDamageEvent {
    pub ixn_event: ProjectileIxnEvent,
    pub attire_entt: Entity,
}
/// Consumes [`ProjectileIxnEvent`]s and damages [`AttireProfile`]s when
/// the object intersecting has one attached.
fn handle_collider_ixn_events(
    mut attires: Query<(Entity, &mut AttireProfile)>,
    mut proj_ixn_events: EventReader<ProjectileIxnEvent>,
    mut pd_events: EventWriter<ProjectileDamageEvent>,
) {
    for event in proj_ixn_events.iter() {
        if let Ok((attire_entt, mut attire)) = attires.get_mut(event.collider.entity()) {
            attire.damage(event.projectile.damage);

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
) {
    for event in coll_dmg_events.iter() {
        tracing::trace!("Collision {:?} | Craft: {:?}", event.damage, event.rb_entt);
    }
    for event in proj_dmg_events.iter() {
        tracing::trace!(
            "Projectile {:?} | Attire: {:?}",
            event.ixn_event.projectile.damage,
            event.attire_entt
        );
    }
}
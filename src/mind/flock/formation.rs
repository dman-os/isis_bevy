use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*, utils::HashMap};

use crate::math::*;
use crate::mind::*;

use super::FlockMembers;

#[derive(Debug, Clone, Component)]
pub struct CurrentFlockFormation {
    pub formation: Entity,
}

/// A bundle for flock strategies.
#[derive(Bundle)]
pub struct FlockFormationBundle {
    pub pattern: FormationPattern,
    pub slotting_strategy: SlottingStrategy,
    pub slots: FormationSlots,
    pub state: FormationState,
    pub output: FormationOutput,
    pub tag: FlockFormation,
    pub parent: Parent,
}

impl FlockFormationBundle {
    pub fn new(
        pattern: FormationPattern,
        slotting_strategy: SlottingStrategy,
        flock_entt: Entity,
    ) -> Self {
        Self {
            pattern,
            slotting_strategy,
            slots: Default::default(),
            state: Default::default(),
            output: Default::default(),
            tag: FlockFormation::new(flock_entt),
            parent: Parent(flock_entt),
        }
    }
}

#[derive(Debug, Clone, Copy, Component)]
pub struct FlockFormation {
    flock_entt: Entity,
}

#[derive(Debug, Component)]
#[component(storage = "SparseSet")]
pub struct ActiveFlockFormation;

impl FlockFormation {
    pub fn new(flock_entt: Entity) -> Self {
        Self { flock_entt }
    }

    /// Get a reference to the flock formation's flock entt.
    pub fn flock_entt(&self) -> &Entity {
        &self.flock_entt
    }
}

#[derive(Debug, Clone, Component)]
pub enum FormationPattern {
    Sphere {
        center: FormationPivot,
        radius: TReal,
    },
}

#[derive(Debug, Clone)]
pub enum FormationPivot {
    Craft { entt: Entity },
    Anchor { xform: Transform },
}

#[derive(Debug, Clone, Component)]
pub enum SlottingStrategy {
    Simple,
}

impl SlottingStrategy {
    pub fn slot(
        &self,
        members: &flock::FlockMembers,
        // _member_xforms: &'a dyn Fn(Entity) -> &'a GlobalTransform,
    ) -> FormationSlots {
        FormationSlots {
            slots: members.iter().enumerate().map(|(i, c)| (*c, i)).collect(),
        }
    }
}

#[derive(Debug, Default, Component)]
pub struct FormationSlots {
    pub slots: HashMap<Entity, usize>,
}

#[derive(Debug, Component, Default)]
pub struct FormationState {
    pub boid_strategies: HashMap<Entity, Entity>,
}

#[derive(Debug, Default, Component)]
pub struct FormationOutput {
    pub positions: HashMap<Entity, TVec3>,
}

#[derive(Default)]
pub struct FormationButlerCache {
    boid_strategies: HashMap<Entity, Entity>,
    positions: HashMap<Entity, TVec3>,
}

#[allow(clippy::type_complexity)]
pub fn butler(
    mut commands: Commands,
    mut formations: QuerySet<(
        QueryState<
            (
                Entity,
                &FlockFormation,
                &SlottingStrategy,
                &mut FormationSlots,
                &mut FormationOutput,
            ),
            Added<FlockFormation>,
        >, // added
        QueryState<
            (&FlockFormation, &SlottingStrategy, &mut FormationSlots),
            Changed<SlottingStrategy>,
        >, // added
        QueryState<(
            &SlottingStrategy,
            &mut FormationState,
            &mut FormationSlots,
            &mut FormationOutput,
        )>, // all
    )>,
    flocks: Query<&FlockMembers>,
    crafts: Query<(&GlobalTransform,)>,
    member_changes: Query<(Entity, &FlockMembers), Changed<FlockMembers>>,
    mut cache: Local<FormationButlerCache>,
) {
    // init new formations
    for (entt, formation, slotting_strategy, mut slots, mut output) in formations.q0().iter_mut() {
        let members = flocks
            .get(formation.flock_entt)
            .expect("unable to find Flock for Formation");

        for craft_entt in members.iter() {
            let craft_entt = *craft_entt;

            let (xform,) = crafts
                .get(craft_entt)
                .expect("unable to find craft for flock");
            output.positions.insert(craft_entt, xform.translation);
            // *directive = boid::BoidMindDirective::JoinFomation { formation: entt };
        }

        *slots = slotting_strategy.slot(members);

        commands.entity(entt).insert(ActiveFlockFormation);
    }
    // handle slotting strategy changes
    for (formation, slotting_strategy, mut slots) in formations.q1().iter_mut() {
        let members = flocks
            .get(formation.flock_entt)
            .expect("unable to find Flock for Formation");

        *slots = slotting_strategy.slot(members);
    }

    // handle flock membership changes
    for (flock_entt, members) in member_changes.iter() {
        if let Ok((slotting_strategy, mut state, mut slots, mut output)) =
            formations.q2().get_mut(flock_entt)
        {
            std::mem::swap(&mut cache.boid_strategies, &mut state.boid_strategies);
            std::mem::swap(&mut cache.positions, &mut output.positions);
            // reset the state while retaining some for pre-existing members
            for craft_entt in members.iter() {
                let craft_entt = *craft_entt;
                if let Some(e) = cache.boid_strategies.remove(&craft_entt) {
                    state.boid_strategies.insert(craft_entt, e);
                    output.positions.insert(
                        craft_entt,
                        cache
                            .positions
                            .remove(&craft_entt)
                            // .unwrap_or_else(|| Default::default())
                            .unwrap(),
                    );
                } else {
                    let (xform,) = crafts
                        .get(craft_entt)
                        .expect("unable to find craft for flock");
                    output.positions.insert(craft_entt, xform.translation);
                    // *directive = boid::BoidMindDirective::JoinFomation { formation: entt };
                }
            }
            cache.boid_strategies.clear();
            cache.positions.clear();
            *slots = slotting_strategy.slot(members);
        }
    }
}

pub fn update(
    mut formations: Query<
        (
            &FormationPattern,
            &FormationSlots,
            &mut FormationState,
            &mut FormationOutput,
        ),
        With<ActiveFlockFormation>,
    >,
    // flocks: Query<&FlockMembers>,
    // crafts: Query<&GlobalTransform>,
) {
    for (pattern, slots, _state, mut out) in formations.iter_mut() {
        let member_size = slots.slots.len();
        if member_size == 0 {
            continue;
        }
        match &pattern {
            FormationPattern::Sphere { center, radius } => {
                let center_pivot = match center {
                    // FormationPivot::Craft { entt } => crafts.get(*entt).unwrap().into(),
                    FormationPivot::Craft { .. } => todo!(),
                    FormationPivot::Anchor { xform } => xform,
                };
                let rays = crate::utils::points_on_sphere(member_size);
                for (craft_entt, ii) in slots.slots.iter() {
                    out.positions.insert(
                        *craft_entt,
                        center_pivot.translation + (rays[*ii] * *radius),
                    );
                }
            }
        }
    }
}

/*

/// A generic bundle for flock strategies.
#[derive(Bundle)]
pub struct FlockFormationBundle<P>
where
    P: Component,
{
    pub param: P,
    pub tag: FlockFormation,
}

impl<P> FlockFormationBundle<P>
where
    P: Component,
{
    pub fn new(param: P, flock_entt: Entity) -> Self {
        Self {
            param,
            tag: FlockFormation::new(flock_entt, FlockFormationKind::of::<P>()),
        }
    }
}

/// A variant of [`FlockFormationBundle`] with two parameter components.
#[derive(Bundle)]
pub struct FlockFormationBundleExtra<P, P2>
where
    P: Component,
    P2: Component,
{
    pub param: P,
    pub extra: P2,
    pub tag: FlockFormation,
}

impl<P, P2> FlockFormationBundleExtra<P, P2>
where
    P: Component,
    P2: Component,
{
    pub fn new(param: P, flock_entt: Entity, extra: P2) -> Self {
        Self {
            param,
            extra,
            tag: FlockFormation::new(flock_entt, FlockFormationKind::of::<P>()),
        }
    }
}

/// A variant of [`FlockFormationBundleExtra`] where the second component is also a bundle.
#[derive(Bundle)]
pub struct FlockFormationBundleJumbo<P, B>
where
    P: Component,
    B: Bundle,
{
    pub param: P,
    #[bundle]
    pub extra: B,
    pub tag: FlockFormation,
}

impl<P, B> FlockFormationBundleJumbo<P, B>
where
    P: Component,
    B: Bundle,
{
    pub fn new(param: P, flock_entt: Entity, extra: B) -> Self {
        Self {
            param,
            extra,
            tag: FlockFormation::new(flock_entt, FlockFormationKind::of::<P>()),
        }
    }
}

pub type FlockFormationKind = std::any::TypeId; */

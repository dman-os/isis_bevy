use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};
use bevy_inspector_egui::Inspectable;

use crate::craft::*;
use crate::mind::*;

pub mod attack_persue;
pub mod custom;
pub mod form;
pub mod run_circuit;

#[derive(Debug, Component)]
#[component(storage = "SparseSet")]
pub struct ActiveBoidStrategy;

/// A generic bundle for craft strategies.
#[derive(Bundle)]
pub struct BoidStrategyBundle<P>
where
    P: Component,
{
    pub param: P,
    pub output: BoidStrategyOutput,
    pub tag: BoidStrategy,
    pub name: Name,
    pub parent: Parent,
}

impl<P> BoidStrategyBundle<P>
where
    P: Component,
{
    // pub const DEFAULT_NAME: &'static str = std::any::type_name::<P>();
    pub const DEFAULT_NAME: &'static str = "boid_strategy";
    pub fn new(param: P, craft_entt: Entity) -> Self {
        Self {
            param,
            output: Default::default(),
            tag: BoidStrategy::new(craft_entt, BoidStrategyKind::of::<P>()),
            name: Self::DEFAULT_NAME.into(),
            parent: Parent(craft_entt),
        }
    }
}

/// A variant of [`BoidStrategyBundle`] with two parameter components.
#[derive(Bundle)]
pub struct BoidStrategyBundleExtra<P, P2>
where
    P: Component,
    P2: Component,
{
    pub param: P,
    pub extra: P2,
    pub output: BoidStrategyOutput,
    pub tag: BoidStrategy,
    pub name: Name,
    pub parent: Parent,
}

impl<P, P2> BoidStrategyBundleExtra<P, P2>
where
    P: Component,
    P2: Component,
{
    pub fn new(param: P, craft_entt: Entity, extra: P2) -> Self {
        Self {
            param,
            output: Default::default(),
            extra,
            tag: BoidStrategy::new(craft_entt, BoidStrategyKind::of::<P>()),
            name: BoidStrategyBundle::<P>::DEFAULT_NAME.into(),
            parent: Parent(craft_entt),
        }
    }
}

/// A variant of [`BoidStrategyDuoComponent`] where the second component is also a bundle.
#[derive(Bundle)]
pub struct BoidStrategyBundleJumbo<P, B>
where
    P: Component,
    B: Bundle,
{
    pub param: P,
    #[bundle]
    pub extra: B,
    pub output: BoidStrategyOutput,
    pub tag: BoidStrategy,
    pub name: Name,
    pub parent: Parent,
}

impl<P, B> BoidStrategyBundleJumbo<P, B>
where
    P: Component,
    B: Bundle,
{
    pub fn new(param: P, craft_entt: Entity, extra: B) -> Self {
        Self {
            param,
            output: Default::default(),
            extra,
            tag: BoidStrategy::new(craft_entt, BoidStrategyKind::of::<P>()),
            name: BoidStrategyBundle::<P>::DEFAULT_NAME.into(),
            parent: Parent(craft_entt),
        }
    }
}

#[derive(Debug, Clone, Default, Inspectable, Component)]
pub struct BoidStrategyOutput {
    #[inspectable(ignore)]
    pub routine_usage: super::SteeringRoutineComposer,
    pub fire_weapons: bool,
}

pub type BoidStrategyKind = std::any::TypeId;

#[derive(Debug, Clone, Copy, Component)]
pub struct BoidStrategy {
    craft_entt: Entity,
    kind: BoidStrategyKind,
}

impl BoidStrategy {
    pub fn new(craft_entt: Entity, kind: BoidStrategyKind) -> Self {
        Self { craft_entt, kind }
    }

    /// Get a reference to the craft strategy's craft entt.
    pub fn craft_entt(&self) -> Entity {
        self.craft_entt
    }

    /// Get a reference to the craft strategy's kind.
    pub fn kind(&self) -> BoidStrategyKind {
        self.kind
    }
}

#[derive(Debug, Default, Clone, Component)]
pub struct CurrentBoidStrategy {
    pub strategy: Option<Entity>,
}

/// This system assigns the [`SteeringRoutineComposer`] emitted by the strategy to the craft
/// and fires weapon.
/// TODO: use change tracking to avoid work
pub fn craft_boid_strategy_output_mgr(
    mut crafts: Query<(
        &mut boid::steering::SteeringRoutineComposer,
        &CurrentBoidStrategy,
        &sensors::CraftWeaponsIndex,
    )>,
    strategies: Query<&BoidStrategyOutput>,
    mut activate_wpn_events: EventWriter<arms::ActivateWeaponEvent>,
    weapons: Query<&arms::WeaponActivationState>,
    time: Res<Time>,
) {
    for (mut composer, mind, wpn_index) in crafts.iter_mut() {
        let strategy = match mind.strategy {
            Some(s) => s,
            None => continue,
        };
        let output = strategies
            .get(strategy)
            .expect("active BoidStrategy not found");
        *composer = output.routine_usage.clone(); // FIXME:

        if output.fire_weapons {
            for wpn in wpn_index.entt_to_class.keys() {
                if weapons
                    .get(*wpn)
                    .expect("Indexed weapon has no WeaponActivationState")
                    .can_activate(&time)
                {
                    activate_wpn_events.send(arms::ActivateWeaponEvent { weapon_id: *wpn });
                }
            }
        }
    }
}

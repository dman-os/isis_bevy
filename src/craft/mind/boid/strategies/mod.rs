use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};
use bevy_inspector_egui::Inspectable;

pub use attack_persue::*;
mod attack_persue;

pub use run_circuit::*;
mod run_circuit;

pub use single_routine::*;
mod single_routine;

/// A generic bundle for craft strategies.
#[derive(Bundle)]
pub struct BoidStrategyBundle<P>
where
    P: Component,
{
    pub param: P,
    pub output: BoidStrategyOutput,
    pub tag: BoidStrategy,
}

impl<P> BoidStrategyBundle<P>
where
    P: Component,
{
    pub fn new(param: P, craft_entt: Entity) -> Self {
        Self {
            param,
            output: Default::default(),
            tag: BoidStrategy::new(craft_entt, BoidStrategyKind::of::<P>()),
        }
    }
}

/// A generic bundle for craft strategies.
#[derive(Bundle)]
pub struct BoidStrategyDuoComponent<P, P2>
where
    P: Component,
    P2: Component,
{
    pub param: P,
    pub extra: P2,
    pub output: BoidStrategyOutput,
    pub tag: BoidStrategy,
}

impl<P, P2> BoidStrategyDuoComponent<P, P2>
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
        }
    }
}

/// A generic bundle for craft strategies.
#[derive(Bundle)]
pub struct BoidStrategyDuoBundle<P, B>
where
    P: Component,
    B: Bundle,
{
    pub param: P,
    #[bundle]
    pub extra: B,
    pub output: BoidStrategyOutput,
    pub tag: BoidStrategy,
}

impl<P, B> BoidStrategyDuoBundle<P, B>
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
        }
    }
}

#[derive(Debug, Clone, Default, Inspectable, Component)]
pub struct BoidStrategyOutput {
    #[inspectable(ignore)]
    pub routine_usage: SteeringRoutineComposer,
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

use super::SteeringRoutineComposer;

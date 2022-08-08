use deps::*;

use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;

pub mod cas;
pub mod form_up;

#[derive(Debug, Clone, Default, Component, Reflect, Inspectable)]
pub struct CurrentFlockStrategy {
    pub strategy: Option<Entity>,
}

#[derive(Debug, Component, Default)]
#[component(storage = "SparseSet")]
pub struct ActiveFlockStrategy;

/// A generic bundle for flock strategies.
#[derive(Bundle)]
pub struct FlockStrategyBundle<P>
where
    P: Component,
{
    pub param: P,
    pub name: Name,
    pub tag: FlockStrategy,
    // pub parent: Parent,
}

impl<P> FlockStrategyBundle<P>
where
    P: Component,
{
    pub const DEFAULT_NAME: &'static str = "flock_strategy";
    pub fn new(param: P, flock_entt: Entity) -> Self {
        Self {
            param,
            tag: FlockStrategy::new(flock_entt, FlockStrategyKind::of::<P>()),
            // parent: Parent(flock_entt),
            name: Self::DEFAULT_NAME.into(),
        }
    }
}

/// A variant of [`FlockStrategyBundle`] with two parameter components.
#[derive(Bundle)]
pub struct FlockStrategyBundleExtra<P, P2>
where
    P: Component,
    P2: Component,
{
    pub param: P,
    pub extra: P2,
    pub name: Name,
    pub tag: FlockStrategy,
    // pub parent: Parent,
}

impl<P, P2> FlockStrategyBundleExtra<P, P2>
where
    P: Component,
    P2: Component,
{
    pub const DEFAULT_NAME: &'static str = "flock_strategy";
    pub fn new(param: P, flock_entt: Entity, extra: P2) -> Self {
        Self {
            param,
            extra,
            tag: FlockStrategy::new(flock_entt, FlockStrategyKind::of::<P>()),
            // parent: Parent(flock_entt),
            name: Self::DEFAULT_NAME.into(),
        }
    }
}

/// A variant of [`FlockStrategyBundleExtra`] where the second component is also a bundle.
#[derive(Bundle)]
pub struct FlockStrategyBundleJumbo<P, B>
where
    P: Component,
    B: Bundle,
{
    pub param: P,
    #[bundle]
    pub extra: B,
    pub name: Name,
    pub tag: FlockStrategy,
    // pub parent: Parent,
}

impl<P, B> FlockStrategyBundleJumbo<P, B>
where
    P: Component,
    B: Bundle,
{
    pub const DEFAULT_NAME: &'static str = "flock_strategy";
    pub fn new(param: P, flock_entt: Entity, extra: B) -> Self {
        Self {
            param,
            extra,
            tag: FlockStrategy::new(flock_entt, FlockStrategyKind::of::<P>()),
            // parent: Parent(flock_entt),
            name: Self::DEFAULT_NAME.into(),
        }
    }
}

pub type FlockStrategyKind = std::any::TypeId;

#[derive(Debug, Clone, Copy, Component)]
pub struct FlockStrategy {
    pub flock_entt: Entity,
    pub kind: FlockStrategyKind,
}

impl FlockStrategy {
    pub fn new(flock_entt: Entity, kind: FlockStrategyKind) -> Self {
        Self { flock_entt, kind }
    }

    pub fn flock_entt(&self) -> Entity {
        self.flock_entt
    }

    pub fn kind(&self) -> FlockStrategyKind {
        self.kind
    }
}

use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};
use educe::Educe;

pub mod strategy;
use strategy::*;

#[derive(Debug, Default, Component, Educe)]
#[educe(Deref, DerefMut)]
pub struct FlockMembers(pub smallvec::SmallVec<[Entity; 8]>);

#[derive(Bundle)]
pub struct FlockMindBundle {
    pub members: FlockMembers,
    // pub events: FlockChangeEvents,
    // smarts layer coordination
    pub active_strategy: CurrentFlockStrategy,
}

impl FlockMindBundle {
    pub fn new(strategy: Entity) -> Self {
        Self {
            members: Default::default(),
            // events: Default::default(),
            active_strategy: CurrentFlockStrategy { strategy },
        }
    }
}

#[derive(Debug, Clone, Copy, Component)]
pub struct CraftFlock(pub Entity);
/*
#[derive(Debug, Default, Component)]
pub struct FlockChangeEvents {
    pub events: smallvec::SmallVec<[FlockChange; 2]>,
}

#[derive(Debug, Component)]
pub enum FlockChange {
    MemberChange {},
}

pub fn flock_change_event_emitter(
    mut flocks: Query<&mut FlockChangeEvents>
) */

/* use formation::*;
pub mod formation {
    use deps::*;

    use bevy::{ecs as bevy_ecs, prelude::*, utils::*};

    use super::*;
    use crate::craft::mind::*;
    use crate::math::*;

    #[derive(Debug, Clone, Component)]
    pub struct ActiveFlockFormation {
        pub formation: Entity,
    }

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

    pub type FlockFormationKind = std::any::TypeId;

    #[derive(Debug, Clone, Copy, Component)]
    pub struct FlockFormation {
        pub flock_entt: Entity,
        pub kind: FlockFormationKind,
    }

    impl FlockFormation {
        pub fn new(flock_entt: Entity, kind: FlockFormationKind) -> Self {
            Self { flock_entt, kind }
        }

        /// Get a reference to the flock formation's flock entt.
        pub fn flock_entt(&self) -> &Entity {
            &self.flock_entt
        }

        /// Get a reference to the flock formation's kind.
        pub fn kind(&self) -> FlockFormationKind {
            self.kind
        }
    }

} */

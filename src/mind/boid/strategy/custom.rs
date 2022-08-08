use deps::*;

use bevy::prelude::*;

use super::{ActiveBoidStrategy, BoidStrategy, BoidStrategyBundle, BoidStrategyOutput};
use crate::mind::boid::steering::*;

pub type RoutineSpawner =
    dyn FnOnce(&mut Commands, &BoidStrategy) -> Entity + Sync + 'static + Send;

pub enum Composition {
    Single {
        routine_spawner: Box<RoutineSpawner>,
    },
    // Linear sum of the routine outputs
    WeightSummed {
        routines: SVec<[(compose::SteeringRoutineWeight, Box<RoutineSpawner>); 2]>,
    },
    /// The first routine that returns a non zero value will be used.
    PriorityOverride {
        routines: SVec<[Box<RoutineSpawner>; 4]>,
    },
}

// pub type Spawner = std::sync::Arc<std::sync::Mutex<dyn FnOnce(&mut Commands) -> Entity>>;
#[derive(Component)]
pub struct Custom {
    composition: Option<Composition>,
}

impl Custom {
    pub fn new(composition: Composition) -> Self {
        Self {
            composition: Some(composition),
        }
    }
}

pub type Bundle = BoidStrategyBundle<Custom>;

pub fn butler(
    mut commands: Commands,
    mut added_strategies: Query<
        (Entity, &mut Custom, &BoidStrategy, &mut BoidStrategyOutput),
        Added<Custom>,
    >,
) {
    for (entt, mut param, strategy, mut out) in added_strategies.iter_mut() {
        let composer = match param.composition.take().unwrap_or_log() {
            Composition::Single { routine_spawner } => {
                let routine = routine_spawner(&mut commands, strategy);

                compose::SteeringRoutineComposer::Single { entt: routine }
            }
            Composition::WeightSummed { routines } => {
                let routines: SVec<[(compose::SteeringRoutineWeight, Entity); 2]> = routines
                    .into_iter()
                    .map(|(weight, spawner)| (weight, spawner(&mut commands, strategy)))
                    .collect();

                compose::SteeringRoutineComposer::WeightSummed { routines }
            }
            Composition::PriorityOverride { routines } => {
                let routines: SVec<[Entity; 4]> = routines
                    .into_iter()
                    .map(|spawner| spawner(&mut commands, strategy))
                    .collect();

                compose::SteeringRoutineComposer::PriorityOverride { routines }
            }
        };
        let compose = commands.entity(strategy.boid_entt()).add_children(|p| {
            p.spawn()
                .insert_bundle(compose::Bundle::new(
                    compose::Compose { composer },
                    strategy.boid_entt(),
                ))
                .id()
        });
        *out = BoidStrategyOutput {
            steering_routine: Some(compose),
            fire_weapons: false,
        };
        commands.entity(entt).insert(ActiveBoidStrategy);
    }
}

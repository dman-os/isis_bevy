use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};

use super::{BoidStrategy, BoidStrategyBundleExtra, BoidStrategyOutput};
use crate::craft::mind::boid::SteeringRoutineComposer;

pub type RoutineSpawner =
    dyn FnOnce(&mut Commands, &BoidStrategy) -> Entity + Sync + 'static + Send;
// pub type Spawner = std::sync::Arc<std::sync::Mutex<dyn FnOnce(&mut Commands) -> Entity>>;
#[derive(Component)]
pub struct SingleRoutine {
    routine_spawner: Option<Box<RoutineSpawner>>,
}

impl SingleRoutine {
    pub fn new(routine_spawner: Box<RoutineSpawner>) -> Self {
        Self {
            routine_spawner: Some(routine_spawner),
        }
    }
}

#[derive(Debug, Clone, Default, Component)]
pub struct SingleRoutineState {
    pub routine: Option<Entity>,
}

pub type SingleRoutineBundle = BoidStrategyBundleExtra<SingleRoutine, SingleRoutineState>;

pub fn single_routine_butler(
    mut commands: Commands,
    mut added_strategies: Query<
        (
            &mut SingleRoutine,
            &BoidStrategy,
            &mut SingleRoutineState,
            &mut BoidStrategyOutput,
        ),
        Added<SingleRoutine>,
    >,
) {
    for (mut params, strategy, mut state, mut out) in added_strategies.iter_mut() {
        let spawner = params.routine_spawner.take().unwrap();
        let routine = spawner(&mut commands, strategy);

        commands
            .entity(strategy.craft_entt())
            .push_children(&[routine]);
        state.routine = Some(routine);
        *out = BoidStrategyOutput {
            routine_usage: SteeringRoutineComposer::Single { entt: routine },
            fire_weapons: false,
        };
    }
}

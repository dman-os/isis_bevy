use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};

use super::{BoidStrategy, BoidStrategyDuoComponent, BoidStrategyOutput};
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

pub type SingleRoutineBundle = BoidStrategyDuoComponent<SingleRoutine, SingleRoutineState>;

pub fn single_routine_butler(
    mut commands: Commands,
    mut added_strategies: Query<
        (&mut SingleRoutine, &BoidStrategy, &mut SingleRoutineState),
        Added<SingleRoutine>,
    >,
) {
    for (mut params, strategy, mut state) in added_strategies.iter_mut() {
        let spawner = params.routine_spawner.take().unwrap();
        let routine = spawner(&mut commands, strategy);

        commands
            .entity(strategy.craft_entt())
            .push_children(&[routine]);
        state.routine = Some(routine);
    }
}

// TODO: active routine filtering
pub fn single_routine(
    mut strategies: Query<
        (&SingleRoutineState, &mut BoidStrategyOutput),
        Changed<SingleRoutineState>,
    >,
) {
    for (state, mut out) in strategies.iter_mut() {
        *out = BoidStrategyOutput {
            routine_usage: SteeringRoutineComposer::Single {
                entt: state.routine.unwrap(),
            },
            fire_weapons: false,
        };
    }
}

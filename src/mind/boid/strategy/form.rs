// FIXME: this layer's just overhead, get rid of it

use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};

use super::{
    super::SteeringRoutineComposer, ActiveBoidStrategy, BoidStrategy, BoidStrategyBundleExtra,
    BoidStrategyOutput,
};
use crate::mind::{boid::steering::*, flock::formation::*, sensors::*};

#[derive(Debug, Clone, Component)]
pub struct Form {
    pub formation: Entity,
}

#[derive(Debug, Clone, Component, Default)]
pub struct FormState {
    pub arrive_routine: Option<Entity>,
    pub avoid_collision_routine: Option<Entity>,
}

pub type Bundle = BoidStrategyBundleExtra<Form, FormState>;

pub fn butler(
    mut commands: Commands,
    mut formations: Query<(&mut FormationState, &FormationOutput)>,
    mut added_strategies: Query<
        (
            Entity,
            &Form,
            &BoidStrategy,
            &mut FormState,
            &mut BoidStrategyOutput,
        ),
        Added<Form>,
    >,
    crafts: Query<&CraftRoutinesIndex>,
) {
    for (entt, param, strategy, mut state, mut out) in added_strategies.iter_mut() {
        let routines = crafts
            .get(strategy.craft_entt())
            .expect("craft not found for BoidStrategy");
        let avoid_collision = routines
            .kind::<avoid_collision::AvoidCollision>()
            .map(|v| v[0])
            .unwrap_or_else(|| {
                commands
                    .spawn()
                    .insert_bundle(avoid_collision::Bundle::new(
                        avoid_collision::AvoidCollision::default(),
                        strategy.craft_entt(),
                    ))
                    .id()
            });

        let (mut formation_state, fomation_output) = formations
            .get_mut(param.formation)
            .expect("Formation not found for Form strategy");
        formation_state
            .boid_strategies
            .insert(strategy.craft_entt(), entt);
        let pos = fomation_output
            .positions
            .get(&strategy.craft_entt())
            .expect("Assigned position not found for formant");
        let pos = *pos;

        let arrive = commands
            .spawn()
            .insert_bundle(arrive::Bundle::new(
                arrive::Arrive {
                    target: arrive::Target::Position { pos },
                    arrival_tolerance: 5.,
                    deceleration_radius: None,
                },
                strategy.craft_entt(),
            ))
            .id();

        state.arrive_routine = Some(arrive);
        state.avoid_collision_routine = Some(avoid_collision);
        *out = BoidStrategyOutput {
            routine_usage: SteeringRoutineComposer::PriorityOverride {
                routines: smallvec::smallvec![avoid_collision, arrive],
            },
            fire_weapons: false,
        };

        commands.entity(entt).insert(ActiveBoidStrategy);
    }
}

pub fn update(
    strategies: Query<(&FormState, Option<&ActiveBoidStrategy>)>,
    formations: Query<(&FormationOutput, &FormationState)>,
    mut arrive_routines: Query<&mut arrive::Arrive>,
) {
    // return;
    for (out, formation_state) in formations.iter() {
        let mut skip_count = 0;

        for (craft_entt, strategy) in formation_state.boid_strategies.iter() {
            let (state, is_active) = strategies
                .get(*strategy)
                .expect("From strategy not found for formant");
            if is_active.is_none() {
                // skip if boid_strategy is not active yet
                skip_count += 1;
                continue;
            }

            let mut arrive_param = arrive_routines
                .get_mut(state.arrive_routine.unwrap())
                .expect("Arrive routine not found for FormState");

            let pos = out
                .positions
                .get(craft_entt)
                .expect("Assigned position not found for formant");
            let pos = *pos;
            arrive_param.target = arrive::Target::Position { pos };
        }
        if skip_count == 0 && out.positions.len() != formation_state.boid_strategies.len() {
            tracing::error!(
                "expected formant count to particpating formant count disrepancies detected"
            );
        }
    }
}

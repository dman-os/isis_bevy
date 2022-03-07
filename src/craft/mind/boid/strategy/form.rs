// FIXME: this layer's just overhead, get rid of it

use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};

use super::{
    super::SteeringRoutineComposer, ActiveBoidStrategy, BoidStrategy, BoidStrategyBundleExtra,
    BoidStrategyOutput,
};
use crate::craft::mind::{boid::steering::*, flock::strategy::*, sensors::*};

#[derive(Debug, Clone, Component)]
pub struct Form {
    pub formation: Entity,
}

#[derive(Debug, Clone, Component, Default)]
pub struct FormState {
    pub arrive_routine: Option<Entity>,
    pub avoid_collision_routine: Option<Entity>,
}

pub type FormBundle = BoidStrategyBundleExtra<Form, FormState>;

pub fn form_butler(
    mut commands: Commands,
    formations: Query<&FormationOutput>,
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
    for (entt, params, strategy, mut state, mut out) in added_strategies.iter_mut() {
        let routines = crafts
            .get(strategy.craft_entt())
            .expect("craft not found for BoidStrategy");
        let avoid_collision = routines
            .kind::<AvoidCollision>()
            .map(|v| v[0])
            .unwrap_or_else(|| {
                commands
                    .spawn()
                    .insert_bundle(AvoidCollisionRoutineBundle::new(
                        AvoidCollision::default(),
                        strategy.craft_entt(),
                    ))
                    .id()
            });

        let fomation_output = formations
            .get(params.formation)
            .expect("Formation not found for Form strategy");
        let pos = fomation_output
            .positions
            .get(&strategy.craft_entt())
            .expect("Assigned position not found for formant");
        let pos = *pos;

        let arrive = commands
            .spawn()
            .insert_bundle(ArriveRoutineBundle::new(
                Arrive {
                    target: ArriveTarget::Position { pos },
                    arrival_tolerance: 5.,
                    deceleration_radius: None,
                },
                strategy.craft_entt(),
            ))
            .id();

        commands
            .entity(strategy.craft_entt())
            .push_children(&[avoid_collision, arrive]);
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

pub fn form(
    strategies: Query<(&FormState, Option<&ActiveBoidStrategy>)>,
    formations: Query<(&FormationOutput, &FormationState)>,
    mut arrive_routines: Query<&mut Arrive>,
) {
    // return;
    for (out, formation_state) in formations.iter() {
        for (craft_entt, strategy) in formation_state.member_to_strategy.iter() {
            let (state, is_active) = strategies
                .get(*strategy)
                .expect("From strategy not found for formant");
            if is_active.is_none() {
                // skip if boid_strategy is not active yet
                continue;
            }

            let mut arrive_params = arrive_routines
                .get_mut(state.arrive_routine.unwrap())
                .expect("Arrive routine not found for FormState");

            let pos = out
                .positions
                .get(craft_entt)
                .expect("Assigned position not found for formant");
            let pos = *pos;
            arrive_params.target = ArriveTarget::Position { pos };
        }
    }
}

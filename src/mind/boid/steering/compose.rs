use deps::*;

use bevy::prelude::*;

use crate::math::*;
use crate::mind::*;

use super::{
    ActiveSteeringRoutine, AngularRoutineOutput, LinAngRoutineBundle, LinearRoutineOutput,
    SteeringRoutine,
};

#[derive(Debug, Clone, Component)]
pub struct Compose {
    pub composer: SteeringRoutineComposer,
}

pub type Bundle = LinAngRoutineBundle<Compose>;

// TODO: run a similar ActiveRoutine tagging for `CurrentSteeringRoutine`s
pub fn butler(
    mut commands: Commands,
    changed: Query<(&Compose, &SteeringRoutine), Changed<Compose>>,
    crafts: Query<(&sensors::SteeringRoutinesIndex,)>,
    mut cache: Local<bevy::utils::HashSet<Entity>>,
) {
    for (param, routine) in changed.iter() {
        let (index,) = crafts.get(routine.boid_entt()).unwrap_or_log();
        // make a set of all the composed routines
        cache.extend(param.composer.all_routines());
        // for all index
        for routine in index.entt_to_kind.keys() {
            // if being composed
            if cache.contains(routine) {
                // these routines are already in index so no modification
                // necessary
                // remove from the update set
                cache.remove(routine);
            } else {
                // deactivate routine
                commands.entity(*routine).remove::<ActiveSteeringRoutine>();
            }
        }
        // for remaining composed routines not in indices
        for entt in cache.drain() {
            commands.entity(entt).insert(ActiveSteeringRoutine);
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn update(
    mut composer_routines: Query<
        (
            &Compose,
            &SteeringRoutine,
            &mut LinearRoutineOutput,
            &mut AngularRoutineOutput,
        ),
        // With<ActiveSteeringRoutine>,
    >,
    other_routines: Query<
        (Option<&LinearRoutineOutput>, Option<&AngularRoutineOutput>),
        (With<SteeringRoutine>, Without<Compose>),
    >,
    boids: Query<(&GlobalTransform,)>,
) {
    for (param, routine, mut lin_out, mut ang_out) in composer_routines.iter_mut() {
        // FIXME: i hate this code
        let active_res = match &param.composer {
            SteeringRoutineComposer::None => Default::default(),
            SteeringRoutineComposer::Single { entt: routine_entt } => {
                match other_routines
                    .get(*routine_entt)
                    .map(|(lin, ang)| BoidSteeringSystemOutput::get_active_res(lin, ang))
                {
                    Ok(Some(res)) => res,
                    Ok(None) => {
                        tracing::error!(
                            ?routine_entt,
                            "Routine doesn't have linear or angular results"
                        );
                        Default::default()
                    }
                    Err(err) => {
                        tracing::error!(
                            ?err,
                            ?routine_entt,
                            "routine not found for ActiveRoutines::Single"
                        );
                        Default::default()
                    }
                }
            }
            SteeringRoutineComposer::WeightSummed { routines: summed } => {
                // zero it out first
                let mut sum = Default::default();
                for (weight, routine_entt) in summed {
                    match other_routines
                        .get(*routine_entt)
                        .map(|(lin, ang)| BoidSteeringSystemOutput::get_active_res(lin, ang))
                    {
                        Ok(Some(res)) => {
                            sum = sum + (*weight * res);
                        }
                        Ok(None) => {
                            tracing::error!(
                                ?routine_entt,
                                "Routine doesn't have linear or angular results"
                            );
                            sum = Default::default();
                            break;
                        }
                        Err(_) => {
                            tracing::error!(
                                ?routine_entt,
                                "routine not found for ActiveRoutines::WeightSummed"
                            );
                            sum = Default::default();
                            break;
                        }
                    }
                }
                sum
            }
            // FIXME: CLEAN ME UP
            SteeringRoutineComposer::PriorityOverride { routines: priority } => {
                // zero it out first
                let mut pick = Default::default();
                'priority_loop: for routine_entt in priority {
                    match other_routines
                        .get(*routine_entt)
                        .map(|(lin, ang)| BoidSteeringSystemOutput::get_active_res(lin, ang))
                    {
                        Ok(Some(res)) => {
                            if !res.is_zero() {
                                // tracing::info!(?res, ?routine_entt, "wasn't zero");
                                pick = res;
                                break 'priority_loop;
                            }
                        }
                        Ok(None) => {
                            tracing::error!(
                                ?routine_entt,
                                "Routine doesn't have linear or angular results"
                            );
                            pick = Default::default();
                            break 'priority_loop;
                        }
                        Err(_) => {
                            tracing::error!(
                                ?routine_entt,
                                "routine not found for ActiveRoutines::PriorityOverride"
                            );
                            pick = Default::default();
                            break 'priority_loop;
                        }
                    }
                }
                pick
            }
            // FIXME: DRY this up
            SteeringRoutineComposer::AvoidCollisionHelper {
                avoid_collision,
                routines: summed,
            } => {
                let avoid_coll_out = match other_routines
                    .get(*avoid_collision)
                    .map(|(lin, ang)| BoidSteeringSystemOutput::get_active_res(lin, ang))
                {
                    Ok(Some(res)) => res,
                    Ok(None) => {
                        tracing::error!(
                            ?avoid_collision,
                            "Routine doesn't have linear or angular results"
                        );
                        Default::default()
                    }
                    Err(err) => {
                        tracing::error!(
                            ?err,
                            ?avoid_collision,
                            "routine not found for ActiveRoutines::AvoidCollisionHelper"
                        );
                        Default::default()
                    }
                };
                if avoid_coll_out.is_zero() {
                    let mut sum = Default::default();
                    for (weight, routine_entt) in summed {
                        match other_routines
                            .get(*routine_entt)
                            .map(|(lin, ang)| BoidSteeringSystemOutput::get_active_res(lin, ang))
                        {
                            Ok(Some(res)) => {
                                sum = sum + (*weight * res);
                            }
                            Ok(None) => {
                                tracing::error!(
                                    ?routine_entt,
                                    "Routine doesn't have linear or angular results"
                                );
                                sum = Default::default();
                                break;
                            }
                            Err(_) => {
                                tracing::error!(
                                    ?routine_entt,
                                    "routine not found for ActiveRoutines::AvoidCollisionHelper"
                                );
                                sum = Default::default();
                                break;
                            }
                        }
                    }
                    sum
                } else {
                    avoid_coll_out
                }
            }
        };
        let (lin, ang) = match active_res {
            BoidSteeringSystemOutput::Both { lin, ang } => (lin, ang),
            BoidSteeringSystemOutput::LinOnly { lin } => {
                // TODO: parameterize this
                let (xform,) = boids.get(routine.boid_entt()).unwrap_or_log();
                // Look at the direction you want to go by default
                (lin, super::look_to(xform.rotation.inverse() * lin))
            }
            BoidSteeringSystemOutput::AngOnly { ang } => (TVec3::ZERO, ang),
        };
        *lin_out = lin.into();
        *ang_out = ang.into();
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SteeringRoutineWeight {
    pub lin: TReal,
    pub ang: TReal,
}

impl Default for SteeringRoutineWeight {
    fn default() -> Self {
        Self { lin: 1., ang: 1. }
    }
}

impl std::ops::Mul<BoidSteeringSystemOutput> for SteeringRoutineWeight {
    type Output = BoidSteeringSystemOutput;

    fn mul(self, rhs: BoidSteeringSystemOutput) -> Self::Output {
        match rhs {
            BoidSteeringSystemOutput::Both {
                lin: rhs_lin,
                ang: rhs_ang,
            } => BoidSteeringSystemOutput::Both {
                lin: rhs_lin * self.lin,
                ang: rhs_ang * self.ang,
            },
            BoidSteeringSystemOutput::LinOnly { lin: rhs_lin } => {
                BoidSteeringSystemOutput::LinOnly {
                    lin: rhs_lin * self.lin,
                }
            }
            BoidSteeringSystemOutput::AngOnly { ang: rhs_ang } => {
                BoidSteeringSystemOutput::AngOnly {
                    ang: rhs_ang * self.ang,
                }
            }
        }
    }
}

impl From<(TReal, TReal)> for SteeringRoutineWeight {
    fn from((lin, ang): (TReal, TReal)) -> Self {
        Self { lin, ang }
    }
}

// Boid mind Component
#[derive(Debug, Clone, Component)]
pub enum SteeringRoutineComposer {
    None,
    Single {
        entt: Entity,
    },
    // Linear sum of the routine outputs
    WeightSummed {
        routines: smallvec::SmallVec<[(SteeringRoutineWeight, Entity); 2]>,
    },
    /// The first routine that returns a non zero value will be used.
    PriorityOverride {
        routines: smallvec::SmallVec<[Entity; 4]>,
    },
    /// A variant of WeightSummed except with a single priority checked routine that goes first
    /// In order to avoid making a second composition layer for the common avoid collision case
    AvoidCollisionHelper {
        avoid_collision: Entity,
        routines: smallvec::SmallVec<[(SteeringRoutineWeight, Entity); 2]>,
    },
}

impl Default for SteeringRoutineComposer {
    fn default() -> Self {
        Self::None
    }
}

impl SteeringRoutineComposer {
    /// This returns a vector of all the routines currently being composed.
    pub fn all_routines(&self) -> smallvec::SmallVec<[Entity; 4]> {
        match self {
            SteeringRoutineComposer::None => Default::default(),
            SteeringRoutineComposer::Single { entt } => smallvec::smallvec![*entt],
            SteeringRoutineComposer::WeightSummed { routines } => {
                routines.iter().map(|(_, entt)| *entt).collect()
            }
            SteeringRoutineComposer::PriorityOverride { routines } => routines.clone(),
            SteeringRoutineComposer::AvoidCollisionHelper {
                avoid_collision,
                routines,
            } => {
                let mut out = smallvec::smallvec![*avoid_collision];
                out.extend(routines.iter().map(|(_, entt)| *entt));
                out
            }
        }
    }
}

/// Contains the engine inputs.
/// Decopling layer between the engine and the minds.
// FIXME: over engineering
// FIXME: this has accured more overengineering somehow
#[derive(Debug, Clone, Copy, educe::Educe)]
#[educe(Default)]
enum BoidSteeringSystemOutput {
    #[educe(Default)]
    Both {
        /// local space
        lin: TVec3,
        /// local space
        ang: TVec3,
    },
    LinOnly {
        /// local space
        lin: TVec3,
    },
    AngOnly {
        /// local space
        ang: TVec3,
    },
}

impl BoidSteeringSystemOutput {
    fn lin(&self) -> TVec3 {
        match self {
            Self::Both { lin, .. } => *lin,
            Self::LinOnly { lin } => *lin,
            Self::AngOnly { .. } => TVec3::ZERO,
        }
    }
    fn ang(&self) -> TVec3 {
        match self {
            Self::Both { ang, .. } => *ang,
            Self::LinOnly { .. } => TVec3::ZERO,
            Self::AngOnly { ang } => *ang,
        }
    }
}

impl std::ops::Add for BoidSteeringSystemOutput {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        match self {
            Self::Both { lin, ang } => match rhs {
                Self::Both {
                    lin: rhs_lin,
                    ang: rhs_ang,
                } => Self::Both {
                    lin: lin + rhs_lin,
                    ang: ang + rhs_ang,
                },
                Self::LinOnly { lin: rhs_lin } => Self::Both {
                    lin: lin + rhs_lin,
                    ang,
                },
                Self::AngOnly { ang: rhs_ang } => Self::Both {
                    lin,
                    ang: ang + rhs_ang,
                },
            },
            Self::LinOnly { lin } => match rhs {
                Self::Both {
                    lin: rhs_lin,
                    ang: rhs_ang,
                } => Self::Both {
                    lin: lin + rhs_lin,
                    ang: rhs_ang,
                },
                Self::LinOnly { lin: rhs_lin } => Self::LinOnly { lin: lin + rhs_lin },
                Self::AngOnly { ang: rhs_ang } => Self::Both { lin, ang: rhs_ang },
            },
            Self::AngOnly { ang } => match rhs {
                Self::Both {
                    lin: rhs_lin,
                    ang: rhs_ang,
                } => Self::Both {
                    lin: rhs_lin,
                    ang: ang + rhs_ang,
                },
                Self::LinOnly { lin: rhs_lin } => Self::Both { lin: rhs_lin, ang },
                Self::AngOnly { ang: rhs_ang } => Self::AngOnly { ang: ang + rhs_ang },
            },
        }
    }
}

impl BoidSteeringSystemOutput {
    #[inline]
    fn is_zero(&self) -> bool {
        self.lin().length_squared() < TReal::EPSILON && self.ang().length_squared() < TReal::EPSILON
    }
    #[inline]
    fn get_active_res(
        lin_res: Option<&LinearRoutineOutput>,
        ang_res: Option<&AngularRoutineOutput>,
    ) -> Option<Self> {
        match (lin_res, ang_res) {
            (Some(lin_res), Some(ang_res)) => Some(Self::Both {
                lin: lin_res.0,
                ang: ang_res.0,
            }),
            (Some(lin_res), None) => {
                let lin = lin_res.0;
                Some(Self::LinOnly { lin })
            }
            (None, Some(ang_res)) => Some(Self::AngOnly { ang: ang_res.0 }),
            (None, None) => None,
        }
    }
}

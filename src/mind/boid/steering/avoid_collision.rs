use deps::*;

use bevy::prelude::*;
use bevy_prototype_debug_lines::*;
use bevy_rapier3d::prelude::*;
use deps::bevy::utils::HashSet;

use super::{
    steering_behaviours, ActiveSteeringRoutine, LinOnlyRoutineBundleExtra, LinearRoutineOutput,
    SteeringRoutine,
};
use crate::craft::attire::*;
use crate::math::*;

#[derive(Debug, Clone, Component)]
pub struct AvoidCollision {
    pub fwd_prediction_secs: f32,
    pub raycast_exclusion: HashSet<Entity>,
    pub upheld_dodge_seconds: f64,
    pub raycast_toi_modifier: TReal,
    pub cast_shape_radius: TReal,
}

impl AvoidCollision {
    pub fn new(cast_shape_radius: TReal, raycast_toi_modifier: TReal) -> Self {
        Self {
            fwd_prediction_secs: 3.0,
            raycast_exclusion: default(),
            upheld_dodge_seconds: 1.5,
            raycast_toi_modifier,
            cast_shape_radius,
        }
    }
}

#[derive(Debug, Clone, Component, Default)]
pub struct AvoidCollisionState {
    /// in world space
    pub flame_dir: TVec3,
    pub linvel: TVec3,
    pub last_dodge_out: LinearRoutineOutput,
    pub last_dodge_timestamp: f64,
    pub craft_colliders: HashSet<Entity>,
}

pub type Bundle = LinOnlyRoutineBundleExtra<AvoidCollision, AvoidCollisionState>;

pub fn butler(
    mut routines: Query<
        (
            &mut AvoidCollisionState,
            &SteeringRoutine,
            ChangeTrackers<SteeringRoutine>,
        ),
        // FIXME: find a way to filter out non-active routines without missing out on changes to Colliders
        // With<ActiveSteeringRoutine>,
    >,
    crafts: Query<(
        &Transform,
        &crate::craft::engine::LinearEngineState,
        &Velocity,
        &crate::Colliders,
        ChangeTrackers<crate::Colliders>,
    )>,
) {
    for (mut state, routine, routine_change) in routines.iter_mut() {
        let (xform, lin_state, vel, colliders, colliders_change) = crafts
            .get(routine.boid_entt())
            .expect_or_log("craft entt not found for routine");

        state.linvel = vel.linvel;
        // state.cast_dir = (state.cast_dir + state.linvel.normalize_or_zero()) * 0.5;

        // use last frame's desired accel dir to cast for obstruction
        state.flame_dir = (xform.rotation * lin_state.input).normalize_or_zero();
        // state.cast_dir = (xform.rotation * lin_state.flame + state.linvel).normalize_or_zero();

        if routine_change.is_added() || colliders_change.is_changed() {
            state.craft_colliders.clear();
            state.craft_colliders.extend(colliders.set.iter());
        }
    }
}

pub fn update(
    // NOTE: this steering system is stateful.
    mut routines: Query<
        (
            &AvoidCollision,
            &mut AvoidCollisionState,
            &SteeringRoutine,
            &mut LinearRoutineOutput,
        ),
        With<ActiveSteeringRoutine>,
    >,
    boids: Query<(&Transform,)>,
    rapier: Res<RapierContext>,
    time: Res<Time>,
    mut lines: ResMut<DebugLines>,
) {
    let mut avoid_collision_raycast_ctr = 0usize;
    for (param, mut state, routine, mut lin_out) in routines.iter_mut() {
        *lin_out = default();
        let (xform,) = boids
            .get(routine.boid_entt())
            .expect_or_log("craft entt not found for routine");

        // let dir = TVec3::from(vel.linvel).normalize();
        let speed = state.linvel.length();
        let toi = param.fwd_prediction_secs * speed;
        let toi = toi + param.raycast_toi_modifier;

        avoid_collision_raycast_ctr += 1;

        let cast_shape = Collider::ball(param.cast_shape_radius);
        // shape rotation matters not for balls
        let cast_pos = xform.translation;
        let cast_rot = xform.rotation;

        lines.line_colored(
            cast_pos,
            cast_pos + state.flame_dir * speed,
            0.,
            Color::ANTIQUE_WHITE,
        );

        lines.line_colored(cast_pos, cast_pos + state.linvel, 0., Color::SEA_GREEN);

        let pred = |handle| {
            // not a craft collider
            !state.craft_colliders.contains(&handle)
                        // not in the exclusion list
                        && !param.raycast_exclusion.contains(&handle)
        };
        let query_filter = QueryFilter {
            groups: Some(InteractionGroups::new(
                ColliderGroups::SOLID.bits(),
                (ColliderGroups::SOLID/* | ColliderGroups::CRAFT_SOLID */).bits(),
            )),
            predicate: Some(&pred),
            ..default()
        };
        let mut hit_dir = None;
        // if collision predicted
        if let Some((handle, hit)) = rapier
            .cast_shape(
                cast_pos,
                cast_rot,
                // world space
                state.flame_dir,
                &cast_shape,
                toi,
                query_filter,
            )
            .map(|val| {
                hit_dir = Some(state.flame_dir);
                val
            })
            .or_else(|| {
                rapier.cast_shape(
                    cast_pos,
                    cast_rot,
                    // world space
                    state.linvel.normalize_or_zero(),
                    &cast_shape,
                    toi,
                    query_filter,
                )
            })
        {
            let hit_dir = hit_dir.unwrap_or_else(|| state.linvel.normalize_or_zero());
            lines.line_colored(
                xform.translation,
                xform.translation + hit_dir * hit.toi,
                0.,
                Color::RED,
            );
            // use behavior to avoid it
            *lin_out = steering_behaviours::avoid_obstacle_seblague(
                hit_dir,
                &mut |cast_dir| {
                    lines.line_colored(
                        xform.translation,
                        xform.translation + cast_dir * toi,
                        0.,
                        Color::BLUE,
                    );
                    avoid_collision_raycast_ctr += 1;
                    rapier
                        .cast_shape(
                            cast_pos,
                            cast_rot,
                            // world space
                            cast_dir,
                            &cast_shape,
                            toi,
                            query_filter,
                        )
                        .is_some()
                },
                xform,
            );
            let dodge_dir = lin_out.get_dir();
            // *lin_out = (dodge_dir - state.linvel.normalize_or_zero()).normalize_or_zero().into();

            lines.line_colored(
                xform.translation,
                xform.translation + dodge_dir * toi,
                0.,
                Color::GREEN,
            );

            // cache avoidance vector
            state.last_dodge_timestamp = time.seconds_since_startup();
            state.last_dodge_out = *lin_out;
            tracing::trace!(
                ?hit_dir,
                ?lin_out,
                ?toi,
                "collision predicted with {handle:?}\n{:?} meters away\n{:?} seconds away\ncorrecting {:?} degrees away",
                hit.toi,
                hit.toi / speed,
                hit_dir.angle_between(dodge_dir) * (180. / crate::math::real::consts::PI)
            );
        }
        // if recently had avoided collision
        else if state.last_dodge_timestamp > 0.0
            && time.seconds_since_startup()
                < (state.last_dodge_timestamp + param.upheld_dodge_seconds)
        {
            let dodge_dir = state.last_dodge_out.get_dir();
            lines.line_colored(
                xform.translation,
                xform.translation + dodge_dir * state.linvel.length(),
                0.,
                Color::GREEN,
            );
            // stick to it until upheld time expires
            *lin_out = state.last_dodge_out;
            // *lin_out = (state.last_dodge_dir - state.linvel.normalize_or_zero()).try_normalize().unwrap_or(state.last_dodge_dir).into();
            lines.line_colored(
                xform.translation,
                xform.translation + dodge_dir * state.linvel.length(),
                0.,
                Color::ORANGE,
            );
        }
    }
    tracing::trace!(avoid_collision_raycast_ctr);
}

use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};
use bevy_inspector_egui::RegisterInspectable;

pub mod sensors;
use sensors::*;
pub mod boid;
use boid::*;
pub mod flock;
pub mod player;
use player::*;

pub struct MindPlugin;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, SystemLabel)]
pub enum CraftMindSystems {
    CraftBoidStrategyOutputMgr,
    SteeringSystems,
    RoutineComposer,
    BoidStrategyButler,
    BoidStrategy,
    FlockStrategyButler,
    FlockStrategy,
    ActiveRoutineTagger,
}

impl Plugin for MindPlugin {
    fn build(&self, app: &mut App) {
        use CraftMindSystems::*;
        app.add_system_to_stage(CoreStage::PostUpdate, wpn_raycaster_butler)
            .init_resource::<CraftWeaponCrossRefIndex>()
            .add_system_to_stage(
                CoreStage::PreUpdate,
                craft_wpn_index_butler.before(BoidStrategyButler),
            )
            .init_resource::<CraftStrategyCrossRefIndex>()
            .add_system_to_stage(
                CoreStage::PreUpdate,
                craft_strategy_index_butler.before(BoidStrategyButler),
            )
            .add_system_set_to_stage(
                // FIXME: we need command flushing between flock strategy butlers and boid strategy butlers
                CoreStage::PreUpdate,
                SystemSet::new()
                    .label(FlockStrategyButler)
                    .with_system(flock::strategy::formation_butler)
                    .with_system(flock::strategy::cas_butler),
            )
            .add_system_set_to_stage(
                CoreStage::PreUpdate,
                SystemSet::new()
                    .label(BoidStrategyButler)
                    .with_system(boid::strategy::attack_persue_butler)
                    .with_system(boid::strategy::run_circuit_butler)
                    .with_system(boid::strategy::form_butler)
                    .with_system(boid::strategy::single_routine_butler),
            )
            .add_system_set(
                SystemSet::new()
                    .label(BoidStrategy)
                    .with_system(boid::strategy::attack_persue)
                    .with_system(boid::strategy::form)
                    .with_system(boid::strategy::run_circuit),
            )
            .add_system_set(
                SystemSet::new()
                    .label(FlockStrategy)
                    .with_system(flock::strategy::cas)
                    .with_system(flock::strategy::formation),
            )
            .add_system(
                craft_boid_strategy_output_mgr
                    .label(CraftBoidStrategyOutputMgr)
                    .after(BoidStrategy),
            )
            .add_system(
                active_routine_tagger
                    .label(ActiveRoutineTagger)
                    .after(CraftBoidStrategyOutputMgr)
                    .before(SteeringSystems),
            )
            .init_resource::<CraftRoutineCrossRefIndex>()
            .add_system(craft_routine_index_butler.after(ActiveRoutineTagger))
            .add_system_set(
                SystemSet::new()
                    .label(SteeringSystems)
                    .with_system(steering::intercept)
                    .with_system(steering::fly_with_flock)
                    .with_system(steering::avoid_collision)
                    .with_system(steering::arrive)
                    .with_system(steering::seek),
            )
            .add_system(
                routine_composer
                    .label(RoutineComposer)
                    .after(SteeringSystems),
            )
            .add_system(mind_update_engine_input.after(RoutineComposer))
            .add_system(cam_input)
            .add_system(engine_input)
            .add_system(wpn_input)
            .add_startup_system(setup_markers)
            .add_system(update_ui_markers)
            .register_inspectable::<CraftCamera>()
            .register_inspectable::<flock::strategy::CASState>()
            .register_inspectable::<BoidMindConfig>()
            .register_inspectable::<BoidSteeringSystemOutput>()
            .register_inspectable::<LinearRoutineOutput>()
            .register_inspectable::<AngularRoutineOutput>()
            .register_inspectable::<steering::AvoidCollision>();
    }
}
/* #[derive(Debug, Clone, Component)]
pub enum ScanPresence {
    Obstacle,
    Boid,
}
 */

/*
use master_mind::*;
mod master_mind {} */

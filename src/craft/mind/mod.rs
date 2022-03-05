use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};
use bevy_inspector_egui::RegisterInspectable;

pub mod sensors;
use sensors::*;
pub mod boid;
use boid::*;
pub mod flock;
use flock::*;
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
                CoreStage::PreUpdate,
                SystemSet::new()
                    .label(BoidStrategyButler)
                    .with_system(strategies::attack_persue_butler)
                    .with_system(strategies::run_circuit_butler)
                    .with_system(strategies::single_routine_butler),
            )
            .add_system_set(
                SystemSet::new()
                    .label(BoidStrategy)
                    .with_system(strategies::attack_persue)
                    .with_system(strategies::single_routine)
                    .with_system(strategies::run_circuit),
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
                    .with_system(steering_systems::intercept)
                    .with_system(steering_systems::fly_with_flock)
                    .with_system(steering_systems::avoid_collision)
                    .with_system(steering_systems::arrive)
                    .with_system(steering_systems::seek),
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
            .add_system(update_flocks)
            .register_inspectable::<CraftCamera>()
            .register_inspectable::<BoidMindConfig>()
            .register_inspectable::<BoidSteeringSystemOutput>()
            .register_inspectable::<LinearRoutineOutput>()
            .register_inspectable::<AngularRoutineOutput>()
            .register_inspectable::<steering_systems::AvoidCollision>();
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

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct MindDrivenCraft;

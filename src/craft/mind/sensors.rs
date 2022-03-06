use deps::*;

use bevy::{
    ecs as bevy_ecs,
    prelude::*,
    utils::{AHashExt, HashMap},
};

use crate::{
    craft::{
        arms::*,
        mind::boid::{steering::*, strategy::*},
    },
    math::*,
};

/// Used to store entity data for [`RemovedComponents`] usage.
#[derive(Debug, Component)]
pub struct CrossReferenceIndex<P> {
    pub data: HashMap<Entity, P>,
}

impl<P> Default for CrossReferenceIndex<P> {
    fn default() -> Self {
        Self {
            data: HashMap::with_capacity(0),
        }
    }
}

impl<P> CrossReferenceIndex<P> {
    pub fn insert(&mut self, entt: Entity, item: P) {
        self.data.insert(entt, item);
    }

    pub fn remove(&mut self, k: &Entity) -> Option<P> {
        self.data.remove(k)
    }
}

/// This'll track all the steering routines currently attached to the craft
/// Craft mind component
#[derive(Debug, Clone, Component, Default)]
pub struct CraftRoutinesIndex {
    pub entt_to_kind: HashMap<Entity, RoutineKind>,
    pub kind_to_entt: HashMap<RoutineKind, smallvec::SmallVec<[Entity; 3]>>,
}

impl CraftRoutinesIndex {
    pub fn kind<P: Component>(&self) -> Option<&smallvec::SmallVec<[Entity; 3]>> {
        self.kind_to_entt.get(&RoutineKind::of::<P>())
    }
    pub fn insert(&mut self, entt: Entity, kind: RoutineKind) {
        self.entt_to_kind.insert(entt, kind);
        self.kind_to_entt.entry(kind).or_default().push(entt);
    }
    pub fn remove(&mut self, entt: Entity) {
        if let Some(kind) = self.entt_to_kind.remove(&entt) {
            if let Some(routines) = self.kind_to_entt.get_mut(&kind) {
                for (ii, entt_in_map) in routines.iter().enumerate() {
                    if *entt_in_map == entt {
                        routines.swap_remove(ii);
                        break;
                    }
                }
            }
        }
    }
}

/// Used to handle index maintainance for the case of [`SteeringRoutine`] entity despawns.
pub type CraftRoutineCrossRefIndex = CrossReferenceIndex<Entity>;

/// This is used to store what craft a previously active routine was pointing to
/// in order to remove the routine from the indice after deactivation.
#[derive(Debug, Clone, Component)]
#[component(storage = "SparseSet")]
pub struct PreviouslyActiveRoutine;

#[allow(clippy::type_complexity)]
pub(super) fn craft_routine_index_butler(
    mut commands: Commands,
    mut routines: QuerySet<(
        // activated
        QueryState<(Entity, &SteeringRoutine), Added<ActiveSteeringRoutine>>,
        // deactivated
        QueryState<
            (Entity, &SteeringRoutine),
            (
                Without<ActiveSteeringRoutine>,
                With<PreviouslyActiveRoutine>,
            ),
        >,
    )>,
    mut indices: Query<&mut CraftRoutinesIndex>,
    removed_components: RemovedComponents<SteeringRoutine>,
    mut cross_ref_index: ResMut<CraftRoutineCrossRefIndex>,
) {
    for (entt, routine) in routines.q0().iter() {
        commands.entity(entt).insert(PreviouslyActiveRoutine);

        // add them to the index
        let mut index = indices
            .get_mut(routine.craft_entt())
            .expect("craft not foud SteeringRoutine");
        index.insert(entt, routine.kind());
    }
    for (entt, routine) in routines.q1().iter() {
        let mut index = indices
            .get_mut(routine.craft_entt())
            .expect("craft_entt not found for ActiveRoutine");
        index.remove(entt);
        commands.entity(entt).remove::<PreviouslyActiveRoutine>();
    }
    for routine in removed_components.iter() {
        if let Some(Ok(mut index)) = cross_ref_index.remove(&routine).map(|e| indices.get_mut(e)) {
            index.remove(routine);
        }
    }
}

/// This'll track all the weapons currently attached to the craft
/// Craft mind component
#[derive(Debug, Clone, Component, Default)]
pub struct CraftWeaponsIndex {
    pub avg_projectile_speed: TReal,
    mean_value_size: usize,
    pub entt_to_class: HashMap<Entity, (WeaponKind, WeaponClass)>,
    pub class_to_entt: HashMap<WeaponClass, smallvec::SmallVec<[Entity; 3]>>,
    pub kind_to_entt: HashMap<WeaponKind, smallvec::SmallVec<[Entity; 3]>>,
}

impl CraftWeaponsIndex {
    pub fn kind<P: Component>(&self) -> Option<&smallvec::SmallVec<[Entity; 3]>> {
        self.kind_to_entt.get(&WeaponKind::of::<P>())
    }
    pub fn insert(&mut self, entt: Entity, kind: WeaponKind, class: WeaponClass) {
        self.entt_to_class.insert(entt, (kind, class));
        self.kind_to_entt.entry(kind).or_default().push(entt);
        self.class_to_entt.entry(class).or_default().push(entt);
    }
    pub fn remove(&mut self, entt: Entity) {
        if let Some((kind, class)) = self.entt_to_class.remove(&entt) {
            if let Some(routines) = self.kind_to_entt.get_mut(&kind) {
                for (ii, entt_in_map) in routines.iter().enumerate() {
                    if *entt_in_map == entt {
                        routines.swap_remove(ii);
                        break;
                    }
                }
            }
            if let Some(routines) = self.class_to_entt.get_mut(&class) {
                for (ii, entt_in_map) in routines.iter().enumerate() {
                    if *entt_in_map == entt {
                        routines.swap_remove(ii);
                        break;
                    }
                }
            }
        }
    }
}

/// Used to handle index maintainance for the case of [`CraftWeapon`] entity despawns.
pub type CraftWeaponCrossRefIndex = CrossReferenceIndex<(Entity, Option<TReal>)>;

pub(super) fn craft_wpn_index_butler(
    new_wpns: Query<(Entity, &CraftWeapon), Added<CraftWeapon>>,
    mut indices: Query<&mut CraftWeaponsIndex>,
    removed: RemovedComponents<CraftWeapon>,
    mut cross_ref_index: ResMut<CraftWeaponCrossRefIndex>,
    projectile_wpns: Query<&ProjectileWeapon>,
) {
    for (entt, wpn) in new_wpns.iter() {
        // add them to the per craft
        let mut index = indices
            .get_mut(wpn.craft_entt())
            .expect("CraftWeaponsIndex not foud craft");
        index.insert(entt, wpn.kind(), wpn.class());

        let speed = if wpn.kind() == WeaponKind::of::<ProjectileWeapon>() {
            let speed = projectile_wpns
                .get(entt)
                .expect("ProjectileWeapon component not found")
                .proj_velocity
                .length();

            index.avg_projectile_speed +=
                (speed - index.avg_projectile_speed) / (index.mean_value_size + 1) as TReal;
            index.mean_value_size += 1;
            Some(speed)
        } else {
            None
        };

        // add them to the global index
        cross_ref_index.insert(entt, (wpn.craft_entt(), speed));
    }
    for removed_wpn in removed.iter() {
        // avoid panicing since the entire craft (and its indices) might be gone
        if let Some((Ok(mut index), speed)) = cross_ref_index
            .remove(&removed_wpn)
            .map(|(e, speed)| (indices.get_mut(e), speed))
        {
            index.remove(removed_wpn);
            if let Some(speed) = speed {
                index.avg_projectile_speed +=
                    (speed - index.avg_projectile_speed) / (index.mean_value_size - 1) as TReal;
                index.mean_value_size -= 1;
            }
        }
    }
}

/// This'll track all the strategies currently attached to the craft
/// Craft mind component
#[derive(Debug, Clone, Component, Default)]
pub struct CraftStrategyIndex {
    pub entt_to_class: HashMap<Entity, BoidStrategyKind>,
    pub kind_to_entt: HashMap<BoidStrategyKind, smallvec::SmallVec<[Entity; 3]>>,
}

impl CraftStrategyIndex {
    pub fn kind<P: Component>(&self) -> Option<&smallvec::SmallVec<[Entity; 3]>> {
        self.kind_to_entt.get(&BoidStrategyKind::of::<P>())
    }
    pub fn insert(&mut self, entt: Entity, kind: BoidStrategyKind) {
        self.entt_to_class.insert(entt, kind);
        self.kind_to_entt.entry(kind).or_default().push(entt);
    }
    pub fn remove(&mut self, entt: Entity) {
        if let Some(kind) = self.entt_to_class.remove(&entt) {
            if let Some(routines) = self.kind_to_entt.get_mut(&kind) {
                for (ii, entt_in_map) in routines.iter().enumerate() {
                    if *entt_in_map == entt {
                        routines.swap_remove(ii);
                        break;
                    }
                }
            }
        }
    }
}

/// Used to handle index maintainance for the case of [`CraftStrategy`] entity despawns.
pub type CraftStrategyCrossRefIndex = CrossReferenceIndex<Entity>;

pub(super) fn craft_strategy_index_butler(
    new: Query<(Entity, &BoidStrategy), Added<BoidStrategy>>,
    mut indices: Query<&mut CraftStrategyIndex>,
    removed: RemovedComponents<BoidStrategy>,
    mut cross_ref_index: ResMut<CraftStrategyCrossRefIndex>,
) {
    for (entt, strategy) in new.iter() {
        // add them to the per craft
        let mut index = indices
            .get_mut(strategy.craft_entt())
            .expect("craft not foud CraftStrategy");
        index.insert(entt, strategy.kind());
        // add them to the global index
        cross_ref_index.insert(entt, strategy.craft_entt());
    }
    for removed_wpn in removed.iter() {
        // avoid panicing since the entire craft (and its indices) might be gone
        if let Some(Ok(mut index)) = cross_ref_index
            .remove(&removed_wpn)
            .map(|e| indices.get_mut(e))
        {
            index.remove(removed_wpn);
        }
    }
}

use deps::*;

use bevy::{prelude::*, utils::HashMap};

use crate::{
    craft::{arms::*, attire::DamageType},
    math::*,
    mind::boid::{steering::*, strategy::*},
};

/// Used to store entity data for [`RemovedComponents`] usage.
#[derive(Debug, Component)]
pub struct CrossReferenceIndex<P> {
    pub index: HashMap<Entity, P>,
}

impl<P> Default for CrossReferenceIndex<P> {
    fn default() -> Self {
        Self {
            index: HashMap::with_capacity(0),
        }
    }
}

impl<P> CrossReferenceIndex<P> {
    pub fn insert(&mut self, entt: Entity, item: P) {
        self.index.insert(entt, item);
    }

    pub fn remove(&mut self, k: &Entity) -> Option<P> {
        self.index.remove(k)
    }
}

/// This'll track all the steering routines currently attached to the craft
/// Craft mind component
#[derive(Debug, Clone, Component, Default)]
pub struct SteeringRoutinesIndex {
    pub entt_to_kind: HashMap<Entity, RoutineKind>,
    pub kind_to_entt: HashMap<RoutineKind, SVec<[Entity; 3]>>,
}

impl SteeringRoutinesIndex {
    pub fn kind<P: Component>(&self) -> Option<&SVec<[Entity; 3]>> {
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
                if routines.is_empty() {
                    self.kind_to_entt.remove(&kind);
                }
            }
        }
    }
}

/// Used to handle index maintainance for the case of [`SteeringRoutine`] entity despawns.
pub type SteeringRoutineCrossRefIndex = CrossReferenceIndex<Entity>;

/// This is used to store what craft a previously active routine was pointing to
/// in order to remove the routine from the indice after deactivation.
#[derive(Debug, Clone, Component)]
#[component(storage = "SparseSet")]
pub struct PreviouslyActiveRoutine;

pub(super) fn craft_routine_index_butler(
    mut commands: Commands,
    mut routines: ParamSet<(
        // activated
        Query<(Entity, &SteeringRoutine), Added<ActiveSteeringRoutine>>,
        // deactivated
        Query<
            (Entity, &SteeringRoutine),
            (
                Without<ActiveSteeringRoutine>,
                With<PreviouslyActiveRoutine>,
            ),
        >,
    )>,
    mut indices: Query<&mut SteeringRoutinesIndex>,
    removed_components: RemovedComponents<SteeringRoutine>,
    mut cross_ref_index: ResMut<SteeringRoutineCrossRefIndex>,
) {
    for (entt, routine) in routines.p0().iter() {
        commands.entity(entt).insert(PreviouslyActiveRoutine);

        // add them to the index
        let mut index = indices
            .get_mut(routine.boid_entt())
            .expect_or_log("craft not foud SteeringRoutine");
        index.insert(entt, routine.kind());
    }
    for (entt, routine) in routines.p1().iter() {
        let mut index = indices
            .get_mut(routine.boid_entt())
            .expect_or_log("boid_entt not found for ActiveRoutine");
        index.remove(entt);
        commands.entity(entt).remove::<PreviouslyActiveRoutine>();
    }
    for routine in removed_components.iter() {
        if let Some(Ok(mut index)) = cross_ref_index.remove(&routine).map(|e| indices.get_mut(e)) {
            index.remove(routine);
        }
    }
}

#[derive(Debug, Clone)]
pub struct WeaponDesc {
    pub kind: WeaponKind,
    pub speed: TReal,
    pub range: TReal,
    pub class: WeaponClass,
    pub damage_type: DamageType,
}

/// This'll track all the weapons currently attached to the craft
/// Craft mind component
#[derive(Debug, Clone, Component, Default)]
pub struct CraftWeaponsIndex {
    pub avg_projectile_speed: TReal,
    mean_value_size: usize,
    pub entt_to_desc: HashMap<Entity, WeaponDesc>,
    pub class_to_entt: HashMap<WeaponClass, SVec<[Entity; 3]>>,
    pub kind_to_entt: HashMap<WeaponKind, SVec<[Entity; 3]>>,
}

impl CraftWeaponsIndex {
    pub fn kind<P: Component>(&self) -> Option<&SVec<[Entity; 3]>> {
        self.kind_to_entt.get(&WeaponKind::of::<P>())
    }
    pub fn insert(&mut self, entt: Entity, desc: WeaponDesc) {
        self.kind_to_entt.entry(desc.kind).or_default().push(entt);
        self.class_to_entt.entry(desc.class).or_default().push(entt);
        self.entt_to_desc.insert(entt, desc);
    }
    pub fn remove(&mut self, entt: Entity) {
        if let Some(WeaponDesc { kind, class, .. }) = self.entt_to_desc.remove(&entt) {
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
pub type CraftWeaponCrossRefIndex = CrossReferenceIndex<(Entity, WeaponDesc)>;

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
            .get_mut(wpn.boid_entt())
            .expect_or_log("CraftWeaponsIndex not found on craft");

        let desc = if WeaponKind::of::<ProjectileWeapon>() == wpn.kind() {
            let param = projectile_wpns
                .get(entt)
                .expect_or_log("ProjectileWeapon component not found");
            let speed = param.proj_velocity.length();

            index.avg_projectile_speed +=
                (speed - index.avg_projectile_speed) / (index.mean_value_size + 1) as TReal;
            index.mean_value_size += 1;
            WeaponDesc {
                kind: wpn.kind(),
                class: wpn.class(),
                range: speed * param.proj_lifespan_secs as f32,
                damage_type: param.proj_damage.damage_type,
                speed,
            }
        } else {
            unreachable!()
        };
        index.insert(entt, desc.clone());

        // add them to the global index
        cross_ref_index.insert(entt, (wpn.boid_entt(), desc));
    }
    for removed_wpn in removed.iter() {
        // avoid panicing since the entire craft (and its indices) might be gone
        if let Some((Ok(mut index), WeaponDesc { speed, .. })) = cross_ref_index
            .remove(&removed_wpn)
            .map(|(e, desc)| (indices.get_mut(e), desc))
        {
            index.remove(removed_wpn);
            index.avg_projectile_speed +=
                (speed - index.avg_projectile_speed) / (index.mean_value_size - 1) as TReal;
            index.mean_value_size -= 1;
        }
    }
}

/// This'll track all the strategies currently attached to the craft
/// Craft mind component
#[derive(Debug, Clone, Component, Default)]
pub struct BoidStrategyIndex {
    pub entt_to_class: HashMap<Entity, BoidStrategyKind>,
    pub kind_to_entt: HashMap<BoidStrategyKind, SVec<[Entity; 3]>>,
}

impl BoidStrategyIndex {
    pub fn kind<P: Component>(&self) -> Option<&SVec<[Entity; 3]>> {
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
pub type BoidStrategyCrossRefIndex = CrossReferenceIndex<Entity>;

pub(super) fn craft_strategy_index_butler(
    new: Query<(Entity, &BoidStrategy), Added<BoidStrategy>>,
    mut indices: Query<&mut BoidStrategyIndex>,
    removed: RemovedComponents<BoidStrategy>,
    mut cross_ref_index: ResMut<BoidStrategyCrossRefIndex>,
) {
    for (entt, strategy) in new.iter() {
        // add them to the per craft
        let mut index = indices
            .get_mut(strategy.boid_entt())
            .expect_or_log("BoidStrategy's boid_entt not found in world");
        index.insert(entt, strategy.kind());
        // add them to the global index
        cross_ref_index.insert(entt, strategy.boid_entt());
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

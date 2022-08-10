include!(concat!(env!("OUT_DIR"), "/deps.rs"));
pub use bevy::{ecs as bevy_ecs, reflect as bevy_reflect};
pub use color_eyre::eyre;
pub use tracing_unwrap::*;

pub use smallvec::SmallVec as SVec;
pub use svec::SVecWrapper as SVecW;

mod svec {
    #[derive(educe::Educe, bevy::prelude::Component)]
    #[educe(Deref, DerefMut)]
    pub struct SVecWrapper<A: smallvec::Array>(pub smallvec::SmallVec<A>);

    impl<A: smallvec::Array> From<smallvec::SmallVec<A>> for SVecWrapper<A> {
        fn from(inner: smallvec::SmallVec<A>) -> Self {
            Self(inner)
        }
    }

    impl<A: smallvec::Array> Default for SVecWrapper<A> {
        #[inline]
        fn default() -> SVecWrapper<A> {
            Self(Default::default())
        }
    }

    impl<A: smallvec::Array> std::fmt::Debug for SVecWrapper<A>
    where
        A::Item: std::fmt::Debug,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_list().entries(self.iter()).finish()
        }
    }

    impl<A: smallvec::Array> Clone for SVecWrapper<A>
    where
        A::Item: Clone,
    {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }

        fn clone_from(&mut self, source: &Self) {
            self.0.clone_from(&source.0)
        }
    }

    impl<T, A> bevy_inspector_egui::Inspectable for SVecWrapper<A>
    where
        A: smallvec::Array<Item = T>,
        T: bevy_inspector_egui::Inspectable + Default,
    {
        type Attributes = <T as bevy_inspector_egui::Inspectable>::Attributes;

        fn ui(
            &mut self,
            ui: &mut bevy_inspector_egui::egui::Ui,
            options: Self::Attributes,
            context: &mut bevy_inspector_egui::Context,
        ) -> bool {
            let mut changed = false;

            ui.vertical(|ui| {
                // let mut to_delete = None;

                let len = self.len();
                for (i, val) in self.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        /* if utils::ui::label_button(ui, "✖", egui::Color32::RED) {
                            to_delete = Some(i);
                        } */
                        changed |= val.ui(ui, options.clone(), &mut context.with_id(i as u64));
                    });

                    if i != len - 1 {
                        ui.separator();
                    }
                }
                /* ui.vertical_centered_justified(|ui| {
                    if ui.button("+").clicked() {
                        self.push(T::default());
                        changed = true;
                    }
                }); */

                /* if let Some(i) = to_delete {
                    self.remove(i);
                    changed = true;
                } */
            });

            changed
        }

        fn setup(app: &mut bevy::prelude::App) {
            T::setup(app);
        }
    }

    // lifted from bevy_reflect internal impl
    use std::any::Any;

    use bevy::reflect::utility::GenericTypeInfoCell;
    use bevy::reflect::{
        Array, ArrayIter, FromReflect, List, ListInfo, Reflect, ReflectMut, ReflectRef, TypeInfo,
        Typed,
    };

    impl<T: smallvec::Array + Send + Sync + 'static> Array for SVecWrapper<T>
    where
        T::Item: FromReflect + Clone,
    {
        fn get(&self, index: usize) -> Option<&dyn Reflect> {
            if index < self.0.len() {
                Some(&self[index] as &dyn Reflect)
            } else {
                None
            }
        }

        fn get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
            if index < self.0.len() {
                Some(&mut self[index] as &mut dyn Reflect)
            } else {
                None
            }
        }

        fn len(&self) -> usize {
            self.0.len()
        }

        fn iter(&self) -> ArrayIter {
            Array::iter(&self.0)
        }
    }

    impl<T: smallvec::Array + Send + Sync + 'static> List for SVecWrapper<T>
    where
        T::Item: FromReflect + Clone,
    {
        fn push(&mut self, value: Box<dyn Reflect>) {
            let value = value.take::<T::Item>().unwrap_or_else(|value| {
                <T as smallvec::Array>::Item::from_reflect(&*value).unwrap_or_else(|| {
                    panic!(
                        "Attempted to push invalid value of type {}.",
                        value.type_name()
                    )
                })
            });
            self.0.push(value);
        }
    }

    impl<T: smallvec::Array + Send + Sync + 'static> Reflect for SVecWrapper<T>
    where
        T::Item: FromReflect + Clone,
    {
        fn type_name(&self) -> &str {
            std::any::type_name::<Self>()
        }

        fn get_type_info(&self) -> &'static TypeInfo {
            <Self as Typed>::type_info()
        }

        fn into_any(self: Box<Self>) -> Box<dyn Any> {
            self
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }

        fn as_reflect(&self) -> &dyn Reflect {
            self
        }

        fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
            self
        }

        fn apply(&mut self, value: &dyn Reflect) {
            bevy::reflect::list_apply(self, value);
        }

        fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
            *self = value.take()?;
            Ok(())
        }

        fn reflect_ref(&self) -> ReflectRef {
            ReflectRef::List(self)
        }

        fn reflect_mut(&mut self) -> ReflectMut {
            ReflectMut::List(self)
        }

        fn clone_value(&self) -> Box<dyn Reflect> {
            Box::new(List::clone_dynamic(self))
        }

        fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
            bevy::reflect::list_partial_eq(self, value)
        }
    }

    impl<T: smallvec::Array + Send + Sync + 'static> Typed for SVecWrapper<T>
    where
        T::Item: FromReflect + Clone,
    {
        fn type_info() -> &'static TypeInfo {
            static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
            CELL.get_or_insert::<Self, _>(|| TypeInfo::List(ListInfo::new::<Self, T::Item>()))
        }
    }

    impl<T: smallvec::Array + Send + Sync + 'static> FromReflect for SVecWrapper<T>
    where
        T::Item: FromReflect + Clone,
    {
        fn from_reflect(reflect: &dyn Reflect) -> Option<Self> {
            if let ReflectRef::List(ref_list) = reflect.reflect_ref() {
                let mut new_list = SVecWrapper(smallvec::SmallVec::with_capacity(ref_list.len()));
                for field in ref_list.iter() {
                    new_list
                        .0
                        .push(<T as smallvec::Array>::Item::from_reflect(field)?);
                }
                Some(new_list)
            } else {
                None
            }
        }
    }
}

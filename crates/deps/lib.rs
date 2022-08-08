include!(concat!(env!("OUT_DIR"), "/deps.rs"));
pub use bevy::{ecs as bevy_ecs, reflect as bevy_reflect};
pub use color_eyre::eyre;
pub use smallvec::SmallVec as SVec;
pub use tracing_unwrap::*;

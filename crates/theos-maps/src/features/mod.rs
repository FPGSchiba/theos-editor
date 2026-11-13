//! Feature kinds and types of terrain maps (`docs/map-package.md` §5), the feature list the
//! stages build, and the helpers they share to site and name features.

mod model;
mod names;
mod pick;
mod types;

pub use model::{Feature, FeatureHandle, FeatureSet, Geometry, Prop};
pub use names::generated_name;
pub use pick::{interior, pick};
pub use types::TYPES;

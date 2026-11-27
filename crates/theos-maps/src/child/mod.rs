//! Child maps: the parent package a run starts from ([`parent`]) and the frame of the child map
//! in the parent's frame ([`frame`]).

pub mod frame;
pub mod parent;
#[cfg(test)]
pub(crate) mod test_support;
#[cfg(test)]
mod tests;
pub mod values;

pub use frame::{ChildFrame, child_frame, standalone_frame};
pub(crate) use frame::{ClassSource, FrameRule, class_frame_of, describe_frame};
pub use parent::{ChildError, ParentContext, ParentFeature, feature_id};
pub(crate) use values::seed;
pub use values::{ClassFrameValue, ParentFeatureValue, ParentFeatures};

//! Child maps: the parent package a run starts from ([`parent`]) and the frame of the child map
//! in the parent's frame ([`frame`]).

pub mod frame;
pub mod parent;
#[cfg(test)]
mod tests;

pub(crate) use frame::FrameRule;
pub use frame::{ChildFrame, child_frame, standalone_frame};
pub use parent::{ChildError, ParentContext, ParentFeature, feature_id};

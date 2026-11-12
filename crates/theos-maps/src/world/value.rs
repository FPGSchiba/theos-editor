//! Typed non-raster values a world can hold next to its rasters.

use std::any::Any;
use std::fmt::Debug;

/// A non-raster value stored as a layer (a river network, a feature list, ...).
pub trait WorldValue: Any + Debug + Send + Sync {
    /// The name of the value's type; equals [`ValueType::NAME`] of the concrete type.
    fn type_name(&self) -> &'static str;

    /// The value as JSON; hashing sorts the object keys, so their order here does not matter.
    fn canonical_json(&self) -> serde_json::Value;

    /// The value as [`Any`], for downcasting.
    fn as_any(&self) -> &dyn Any;

    /// The value as mutable [`Any`], for downcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// A [`WorldValue`] with a fixed type name, so [`crate::world::World::value`] can find it.
pub trait ValueType: WorldValue + Sized {
    /// The type name; equals `type_name()` of every instance.
    const NAME: &'static str;
}

//! Declarations of the layers a stage reads and writes.

use crate::world::LayerKind;

/// A layer a stage declares as a read or a write: its name, its kind and, for a read, whether
/// it may be absent (spec §5.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerSpec {
    /// The layer name, snake case (spec §5.4).
    pub name: &'static str,
    /// What the layer holds.
    pub kind: LayerKind,
    /// An optional read is not required by validation, and the context returns `None` when the
    /// layer is absent. An optional write may be left out.
    pub optional: bool,
}

impl LayerSpec {
    /// A required layer of `kind`.
    #[must_use]
    pub const fn new(name: &'static str, kind: LayerKind) -> Self {
        Self {
            name,
            kind,
            optional: false,
        }
    }

    /// A float raster.
    #[must_use]
    pub const fn f64(name: &'static str) -> Self {
        Self::new(name, LayerKind::F64)
    }

    /// A byte raster.
    #[must_use]
    pub const fn u8(name: &'static str) -> Self {
        Self::new(name, LayerKind::U8)
    }

    /// A mask.
    #[must_use]
    pub const fn mask(name: &'static str) -> Self {
        Self::new(name, LayerKind::Mask)
    }

    /// A `u32` raster.
    #[must_use]
    pub const fn u32(name: &'static str) -> Self {
        Self::new(name, LayerKind::U32)
    }

    /// A typed value whose type is named `type_name` ([`crate::world::ValueType::NAME`]).
    #[must_use]
    pub const fn value(name: &'static str, type_name: &'static str) -> Self {
        Self::new(name, LayerKind::Value(type_name))
    }

    /// The same layer, optional.
    #[must_use]
    pub const fn optional(self) -> Self {
        Self {
            optional: true,
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LayerSpec;
    use crate::world::LayerKind;

    /// The constructors are `const`, so stages can keep their declarations in constants.
    const UPLIFT: LayerSpec = LayerSpec::f64("uplift").optional();

    #[test]
    fn constructors_set_name_kind_and_optional() {
        let spec = |name, kind| LayerSpec {
            name,
            kind,
            optional: false,
        };
        assert_eq!(LayerSpec::f64("height"), spec("height", LayerKind::F64));
        assert_eq!(LayerSpec::u8("class"), spec("class", LayerKind::U8));
        assert_eq!(LayerSpec::mask("fixed"), spec("fixed", LayerKind::Mask));
        assert_eq!(LayerSpec::u32("stack"), spec("stack", LayerKind::U32));
        assert_eq!(
            LayerSpec::value("climate", "climate"),
            spec("climate", LayerKind::Value("climate"))
        );
        assert!(UPLIFT.optional && UPLIFT.kind == LayerKind::F64);
        assert!(!LayerSpec::f64("height").optional);
    }
}

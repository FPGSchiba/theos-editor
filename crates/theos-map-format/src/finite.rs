//! Finds NaN and infinities in anything `serde` can serialise.
//!
//! JSON cannot hold them, and `serde_json` writes them as `null` without an error, both into a
//! `Value` and into text: by the time a `Value` exists the number is gone. So the check walks the
//! data itself, through `serde`'s `Serializer` trait, before `serde_json` sees it. Nothing is
//! written; every method only looks at what it gets. No maintained crate offers this check, and
//! `serde_json` has no option to refuse the numbers (ruling of the final review, round 1).

use serde::Serialize;
use serde::ser::{
    self, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant,
};

/// Why the walk stopped.
#[derive(Debug)]
pub(crate) enum Stop {
    /// A float is NaN or an infinity.
    NonFinite,
    /// The value's own `Serialize` failed; `serde_json` reports that error itself.
    Other,
}

impl std::fmt::Display for Stop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NonFinite => "a number is not finite",
            Self::Other => "the value could not be serialised",
        })
    }
}

impl std::error::Error for Stop {}

impl ser::Error for Stop {
    fn custom<T: std::fmt::Display>(_: T) -> Self {
        Self::Other
    }
}

/// Whether `value` holds no NaN and no infinity. A value whose own `Serialize` fails counts as
/// finite: the JSON writer reports that failure.
pub(crate) fn is_finite<T: Serialize + ?Sized>(value: &T) -> bool {
    !matches!(value.serialize(Scan), Err(Stop::NonFinite))
}

/// The serializer that walks a value and stops at the first non-finite float.
struct Scan;

/// Accepts a scalar of each listed type.
macro_rules! accept {
    ($($method:ident: $ty:ty),* $(,)?) => {
        $(fn $method(self, _: $ty) -> Result<(), Stop> {
            Ok(())
        })*
    };
}

impl ser::Serializer for Scan {
    type Ok = ();
    type Error = Stop;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    accept!(
        serialize_bool: bool,
        serialize_i8: i8,
        serialize_i16: i16,
        serialize_i32: i32,
        serialize_i64: i64,
        serialize_i128: i128,
        serialize_u8: u8,
        serialize_u16: u16,
        serialize_u32: u32,
        serialize_u64: u64,
        serialize_u128: u128,
        serialize_char: char,
        serialize_str: &str,
        serialize_bytes: &[u8],
        serialize_unit_struct: &'static str,
    );

    fn serialize_f32(self, v: f32) -> Result<(), Stop> {
        if v.is_finite() {
            Ok(())
        } else {
            Err(Stop::NonFinite)
        }
    }

    fn serialize_f64(self, v: f64) -> Result<(), Stop> {
        if v.is_finite() {
            Ok(())
        } else {
            Err(Stop::NonFinite)
        }
    }

    fn serialize_none(self) -> Result<(), Stop> {
        Ok(())
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<(), Stop> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<(), Stop> {
        Ok(())
    }

    fn serialize_unit_variant(self, _: &'static str, _: u32, _: &'static str) -> Result<(), Stop> {
        Ok(())
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<(), Stop> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        value: &T,
    ) -> Result<(), Stop> {
        value.serialize(self)
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Self, Stop> {
        Ok(self)
    }

    fn serialize_tuple(self, _: usize) -> Result<Self, Stop> {
        Ok(self)
    }

    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Self, Stop> {
        Ok(self)
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self, Stop> {
        Ok(self)
    }

    fn serialize_map(self, _: Option<usize>) -> Result<Self, Stop> {
        Ok(self)
    }

    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self, Stop> {
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self, Stop> {
        Ok(self)
    }
}

impl SerializeSeq for Scan {
    type Ok = ();
    type Error = Stop;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Stop> {
        value.serialize(Scan)
    }
    fn end(self) -> Result<(), Stop> {
        Ok(())
    }
}

impl SerializeTuple for Scan {
    type Ok = ();
    type Error = Stop;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Stop> {
        value.serialize(Scan)
    }
    fn end(self) -> Result<(), Stop> {
        Ok(())
    }
}

impl SerializeTupleStruct for Scan {
    type Ok = ();
    type Error = Stop;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Stop> {
        value.serialize(Scan)
    }
    fn end(self) -> Result<(), Stop> {
        Ok(())
    }
}

impl SerializeTupleVariant for Scan {
    type Ok = ();
    type Error = Stop;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Stop> {
        value.serialize(Scan)
    }
    fn end(self) -> Result<(), Stop> {
        Ok(())
    }
}

impl SerializeMap for Scan {
    type Ok = ();
    type Error = Stop;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Stop> {
        key.serialize(Scan)
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Stop> {
        value.serialize(Scan)
    }
    fn end(self) -> Result<(), Stop> {
        Ok(())
    }
}

impl SerializeStruct for Scan {
    type Ok = ();
    type Error = Stop;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        _: &'static str,
        value: &T,
    ) -> Result<(), Stop> {
        value.serialize(Scan)
    }
    fn end(self) -> Result<(), Stop> {
        Ok(())
    }
}

impl SerializeStructVariant for Scan {
    type Ok = ();
    type Error = Stop;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        _: &'static str,
        value: &T,
    ) -> Result<(), Stop> {
        value.serialize(Scan)
    }
    fn end(self) -> Result<(), Stop> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde::Serialize;

    use super::*;

    #[derive(Serialize)]
    enum Shape {
        Unit,
        Point(f64),
        Pair(f32, f64),
        Named { x: f64 },
    }

    #[derive(Serialize)]
    struct Tuple(i8, f64);

    #[derive(Serialize)]
    struct Unit;

    #[derive(Serialize)]
    struct Everything {
        flag: bool,
        ints: (i16, i32, i64, i128, u8, u16, u32, u64, u128),
        letter: char,
        text: String,
        bytes: serde_bytes_like::Bytes,
        none: Option<f64>,
        some: Option<f64>,
        unit: (),
        unit_struct: Unit,
        shapes: Vec<Shape>,
        tuple: Tuple,
        map: BTreeMap<String, f64>,
        nested: Vec<Vec<f32>>,
    }

    mod serde_bytes_like {
        /// Serialises as bytes, as `serde_bytes` would.
        pub struct Bytes(pub Vec<u8>);
        impl serde::Serialize for Bytes {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_bytes(&self.0)
            }
        }
    }

    fn everything(bad: Option<usize>) -> Everything {
        let pick = |i: usize| if bad == Some(i) { f64::NAN } else { 1.0 };
        Everything {
            flag: true,
            ints: (1, 2, 3, 4, 5, 6, 7, 8, 9),
            letter: 'a',
            text: "t".to_owned(),
            bytes: serde_bytes_like::Bytes(vec![1, 2]),
            none: None,
            some: Some(pick(0)),
            unit: (),
            unit_struct: Unit,
            shapes: vec![
                Shape::Unit,
                Shape::Point(pick(1)),
                Shape::Pair(1.0, pick(2)),
                Shape::Named { x: pick(3) },
            ],
            tuple: Tuple(1, pick(4)),
            map: BTreeMap::from([("k".to_owned(), pick(5))]),
            nested: vec![vec![if bad == Some(6) { f32::INFINITY } else { 1.0 }]],
        }
    }

    #[test]
    fn every_shape_of_data_is_walked() {
        assert!(is_finite(&everything(None)));
        for bad in 0..7 {
            assert!(!is_finite(&everything(Some(bad))), "place {bad}");
        }
    }

    #[test]
    fn infinities_are_not_finite_and_negative_zero_is() {
        assert!(!is_finite(&f64::NEG_INFINITY));
        assert!(!is_finite(&[1.0_f32, f32::NAN]));
        assert!(is_finite(&-0.0_f64));
    }

    /// A value whose `Serialize` fails on its own.
    struct Broken;
    impl Serialize for Broken {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("broken"))
        }
    }

    #[test]
    fn a_failing_serialize_is_left_to_the_writer() {
        assert!(is_finite(&Broken));
        assert_eq!(Stop::Other.to_string(), "the value could not be serialised");
        assert_eq!(Stop::NonFinite.to_string(), "a number is not finite");
    }
}

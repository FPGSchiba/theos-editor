//! The rock table: erodibility, diffusivity, critical slope and map colour of every rock.

use serde::{Deserialize, Serialize};

/// The rocks of the geology stage; the discriminant is the value in the `rock` raster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Rock {
    /// Hard crystalline intrusive rock.
    Granite = 0,
    /// Metamorphosed sandstone, hard.
    Quartzite = 1,
    /// Volcanic rock.
    Basalt = 2,
    /// Carbonate sediment rock.
    Limestone = 3,
    /// Clastic sediment rock.
    Sandstone = 4,
    /// Weak fine-grained sediment rock.
    Shale = 5,
    /// Loose sediment of basin floors.
    Alluvium = 6,
}

impl Rock {
    /// Every rock, in raster order.
    pub const ALL: [Rock; 7] = [
        Rock::Granite,
        Rock::Quartzite,
        Rock::Basalt,
        Rock::Limestone,
        Rock::Sandstone,
        Rock::Shale,
        Rock::Alluvium,
    ];

    /// The value of this rock in the `rock` raster.
    #[must_use]
    pub fn code(self) -> u8 {
        self as u8
    }

    /// The rock with raster value `code`, if any.
    #[must_use]
    pub fn from_code(code: u8) -> Option<Rock> {
        Self::ALL.get(usize::from(code)).copied()
    }

    /// Whether this is a hard rock: granite, quartzite or basalt.
    #[must_use]
    pub fn is_hard(self) -> bool {
        matches!(self, Rock::Granite | Rock::Quartzite | Rock::Basalt)
    }

    /// The lower-case name, as in recipes (`rock_mix` keys) and legends.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Rock::Granite => "granite",
            Rock::Quartzite => "quartzite",
            Rock::Basalt => "basalt",
            Rock::Limestone => "limestone",
            Rock::Sandstone => "sandstone",
            Rock::Shale => "shale",
            Rock::Alluvium => "alluvium",
        }
    }
}

/// What the generator and the renders know about a rock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RockProps {
    /// Stream-power erodibility K.
    pub k: f64,
    /// Hillslope diffusivity, m^2/yr.
    pub kd: f64,
    /// Critical slope, the tangent of the threshold hillslope angle.
    pub critical_slope: f64,
    /// Map colour, sRGB.
    pub colour: [u8; 3],
}

/// The properties of `rock`.
///
/// `k`, `kd` and `colour` are 0.3.0's `ROCKS` table. `critical_slope` is the builder's choice
/// of threshold-slope tangent in the range Montgomery & Brandon (2002, "Topographic controls on
/// erosion rates in tectonically active mountain ranges", EPSL 201) use for soil-mantled
/// threshold hillslopes: crystalline rock at about 40 degrees, weak sediment near 25 degrees.
/// The values fall with erodibility, so granite = quartzite >= basalt >= limestone >= sandstone
/// >= shale >= alluvium (spec 5.9).
#[must_use]
pub fn props(rock: Rock) -> RockProps {
    let (k, kd, critical_slope, colour) = match rock {
        Rock::Granite => (1.6e-5, 0.004, 0.84, [0xb8, 0xab, 0xa3]),
        Rock::Quartzite => (1.8e-5, 0.004, 0.84, [0xdd, 0xd5, 0xc6]),
        Rock::Basalt => (2.6e-5, 0.006, 0.80, [0x62, 0x5e, 0x60]),
        Rock::Limestone => (5.0e-5, 0.010, 0.75, [0xd3, 0xcc, 0xb2]),
        Rock::Sandstone => (8.0e-5, 0.012, 0.70, [0xc9, 0xa0, 0x72]),
        Rock::Shale => (2.0e-4, 0.020, 0.60, [0x82, 0x79, 0x6a]),
        Rock::Alluvium => (4.0e-4, 0.030, 0.45, [0xad, 0x9f, 0x7c]),
    };
    RockProps {
        k,
        kd,
        critical_slope,
        colour,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip_in_order() {
        for (i, rock) in Rock::ALL.iter().enumerate() {
            assert_eq!(usize::from(rock.code()), i);
            assert_eq!(Rock::from_code(rock.code()), Some(*rock));
        }
        assert_eq!(Rock::from_code(7), None);
    }

    #[test]
    fn names_are_lower_case_and_unique() {
        let names: Vec<_> = Rock::ALL.iter().map(|r| r.name()).collect();
        assert_eq!(names[0], "granite");
        assert_eq!(names[6], "alluvium");
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 7);
    }

    #[test]
    fn the_table_is_0_3_0s() {
        let p = props(Rock::Shale);
        assert!((p.k - 2.0e-4).abs() < 1e-12 && (p.kd - 0.02).abs() < 1e-12);
        assert_eq!(props(Rock::Basalt).colour, [0x62, 0x5e, 0x60]);
    }
}

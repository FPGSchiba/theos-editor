//! The class table of settlement maps (spec P29 §5.5): the window a map covers around its
//! settlement, the cell edge, and the radius the town grows to, per class. A recipe may
//! override any of the three in `[child.class.<class>]`.

use serde::{Deserialize, Serialize};

/// What a class fixes about a settlement map.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ClassFrame {
    /// The side of the square window the map covers, m.
    pub window_m: f64,
    /// The cell edge, m.
    pub cell_m: f64,
    /// The radius of the built-up area, m.
    pub r_built_m: f64,
}

/// The four classes with their defaults, smallest first.
pub const CLASSES: [(&str, ClassFrame); 4] = [
    (
        "hamlet",
        ClassFrame {
            window_m: 600.0,
            cell_m: 2.0,
            r_built_m: 70.0,
        },
    ),
    (
        "village",
        ClassFrame {
            window_m: 1000.0,
            cell_m: 3.0,
            r_built_m: 170.0,
        },
    ),
    (
        "town",
        ClassFrame {
            window_m: 1600.0,
            cell_m: 4.0,
            r_built_m: 330.0,
        },
    ),
    (
        "city",
        ClassFrame {
            window_m: 2400.0,
            cell_m: 5.0,
            r_built_m: 560.0,
        },
    ),
];

/// The frame of `class`: its default, with the fields `[child.class.<class>]` of the recipe's
/// `child` table sets.
///
/// # Errors
/// Text naming the problem: an unknown class (also as a key of `[child.class]`), an unknown key
/// of the class's table, or a value that is not a positive finite number.
pub fn resolve(class: &str, child: Option<&toml::Table>) -> Result<ClassFrame, String> {
    let known = |name: &str| CLASSES.iter().find(|(n, _)| *n == name).map(|(_, f)| *f);
    let mut frame = known(class).ok_or_else(|| {
        let names: Vec<&str> = CLASSES.iter().map(|(n, _)| *n).collect();
        format!(
            "unknown settlement class `{class}` (known: {})",
            names.join(", ")
        )
    })?;
    let Some(classes) = child.and_then(|c| c.get("class")) else {
        return Ok(frame);
    };
    let classes = classes
        .as_table()
        .ok_or("`child.class` must be a table of classes")?;
    if let Some(name) = classes.keys().find(|n| known(n).is_none()) {
        return Err(format!("`child.class.{name}` names no settlement class"));
    }
    let Some(table) = classes.get(class) else {
        return Ok(frame);
    };
    let table = table
        .as_table()
        .ok_or_else(|| format!("`child.class.{class}` must be a table"))?;
    for (key, value) in table {
        let slot = match key.as_str() {
            "window_m" => &mut frame.window_m,
            "cell_m" => &mut frame.cell_m,
            "r_built_m" => &mut frame.r_built_m,
            other => {
                return Err(format!(
                    "`child.class.{class}.{other}` is not a class key (window_m, cell_m, \
                     r_built_m)"
                ));
            }
        };
        let number = value
            .as_float()
            .or_else(|| value.as_integer().map(num_from_integer))
            .filter(|n| n.is_finite() && *n > 0.0)
            .ok_or_else(|| {
                format!("`child.class.{class}.{key}` must be a positive finite number")
            })?;
        *slot = number;
    }
    Ok(frame)
}

/// A TOML integer as a float; class values are far below 2^53.
#[expect(
    clippy::cast_precision_loss,
    reason = "class values stay far below 2^53"
)]
fn num_from_integer(n: i64) -> f64 {
    n as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(text: &str) -> toml::Table {
        toml::from_str(text).unwrap()
    }

    #[test]
    fn the_defaults_are_the_spec_table() {
        let got: Vec<(f64, f64, f64)> = CLASSES
            .iter()
            .map(|(_, f)| (f.window_m, f.cell_m, f.r_built_m))
            .collect();
        assert_eq!(
            got,
            [
                (600.0, 2.0, 70.0),
                (1000.0, 3.0, 170.0),
                (1600.0, 4.0, 330.0),
                (2400.0, 5.0, 560.0)
            ]
        );
        assert_eq!(resolve("city", None).unwrap(), CLASSES[3].1);
    }

    #[test]
    fn a_class_table_overrides_single_fields() {
        let child = table("[class.town]\ncell_m = 5\nr_built_m = 400.5");
        let town = resolve("town", Some(&child)).unwrap();
        assert_eq!(
            (town.window_m, town.cell_m, town.r_built_m),
            (1600.0, 5.0, 400.5)
        );
        assert_eq!(resolve("hamlet", Some(&child)).unwrap(), CLASSES[0].1);
    }

    #[test]
    fn unknown_classes_and_keys_are_refused() {
        assert!(
            resolve("metropolis", None)
                .unwrap_err()
                .contains("metropolis")
        );
        let bad_key = table("[class.town]\ncell = 5.0");
        assert!(
            resolve("town", Some(&bad_key))
                .unwrap_err()
                .contains("child.class.town.cell")
        );
        let bad_class = table("[class.keep]\ncell_m = 5.0");
        assert!(
            resolve("town", Some(&bad_class))
                .unwrap_err()
                .contains("keep")
        );
        let bad_value = table("[class.town]\ncell_m = -1.0");
        assert!(
            resolve("town", Some(&bad_value))
                .unwrap_err()
                .contains("positive")
        );
        let text_value = table("[class.town]\ncell_m = \"x\"");
        assert!(resolve("town", Some(&text_value)).is_err());
    }
}

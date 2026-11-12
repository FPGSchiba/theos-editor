//! Test stages and recipes for the recipe tests: `a` writes `x`, `b` reads `x` and writes `y`,
//! `c` reads the optional `z`, `o` writes `x` only optionally, `ord` insists on `x` in
//! `check_order`.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use serde::Deserialize;

use super::{LoadedRecipe, RecipeOrigin, parse};
use crate::stage::{
    LayerSpec, MapKind, ParamError, Registry, StageContext, StageError, StageReport, TypedStage,
};

/// Parameters of a stage that takes none.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct Knob {}

macro_rules! test_stage {
    ($name:ident, $id:literal, reads: $reads:expr, writes: $writes:expr) => {
        pub(crate) struct $name;
        impl TypedStage for $name {
            type Params = Knob;
            const ID: &'static str = $id;
            const SUMMARY: &'static str = "A test stage.";
            const KINDS: &'static [MapKind] = &[MapKind::Terrain];
            fn reads(&self, _: &Knob) -> Vec<LayerSpec> {
                $reads
            }
            fn writes(&self, _: &Knob) -> Vec<LayerSpec> {
                $writes
            }
            fn run(&self, _: &mut StageContext<'_>, _: &Knob) -> Result<StageReport, StageError> {
                Ok(StageReport::default())
            }
        }
    };
}

test_stage!(A, "a", reads: vec![], writes: vec![LayerSpec::f64("x")]);
test_stage!(B, "b", reads: vec![LayerSpec::f64("x")], writes: vec![LayerSpec::f64("y")]);
test_stage!(C, "c", reads: vec![LayerSpec::f64("z").optional()], writes: vec![]);
test_stage!(O, "o", reads: vec![], writes: vec![LayerSpec::f64("x").optional()]);

pub(crate) struct Ord;

impl TypedStage for Ord {
    type Params = Knob;
    const ID: &'static str = "ord";
    const SUMMARY: &'static str = "Needs x before it.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];
    fn reads(&self, _: &Knob) -> Vec<LayerSpec> {
        vec![]
    }
    fn writes(&self, _: &Knob) -> Vec<LayerSpec> {
        vec![]
    }
    fn check_order(&self, _: &Knob, available: &BTreeSet<&str>) -> Result<(), ParamError> {
        if available.contains("x") {
            Ok(())
        } else {
            Err(ParamError {
                stage: "ord".to_owned(),
                parameter: Some("process[0]".to_owned()),
                message: "step needs `x`".to_owned(),
            })
        }
    }
    fn run(&self, _: &mut StageContext<'_>, _: &Knob) -> Result<StageReport, StageError> {
        Ok(StageReport::default())
    }
}

/// A registry of the test stages.
pub(crate) fn registry() -> Registry {
    let mut registry = Registry::empty();
    registry.add(Box::new(A));
    registry.add(Box::new(B));
    registry.add(Box::new(C));
    registry.add(Box::new(O));
    registry.add(Box::new(Ord));
    registry
}

/// Recipe text of a 4 x 8 grid of 125 m cells (1 x 0.5 km) with `extra` lines before any table
/// and the stage ids as `[[stage]]` tables.
pub(crate) fn text(extra: &str, stages: &[&str]) -> String {
    let mut text = format!(
        "id = \"t\"\ntitle = \"T\"\nkind = \"terrain\"\ngrid = [4, 8]\nsize_km = [1.0, 0.5]\n{extra}\n"
    );
    for stage in stages {
        let _ = write!(text, "[[stage]]\nid = \"{stage}\"\n");
    }
    text
}

/// Parses [`text`].
pub(crate) fn recipe(extra: &str, stages: &[&str]) -> LoadedRecipe {
    parse(&text(extra, stages), RecipeOrigin::Builtin).unwrap()
}

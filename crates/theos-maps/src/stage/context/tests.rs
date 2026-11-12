use super::{StageContext, StepInfo};
use crate::grid::Grid;
use crate::progress::Progress;
use crate::rng::StageRng;
use crate::stage::test_support::{Counter, Harness, Label, Probe, SEED, Step, Update};
use crate::stage::{Access, Stage, StageError};
use crate::world::{Layer, LayerKind, WorldError};

fn undeclared(stage: &str, layer: &str, access: Access) -> StageError {
    StageError::Undeclared {
        stage: stage.into(),
        layer: layer.into(),
        access,
    }
}

fn missing(layer: &str) -> StageError {
    StageError::Missing {
        stage: "probe".into(),
        layer: layer.into(),
    }
}

fn wrong_kind(layer: &str, expected: LayerKind, found: LayerKind) -> StageError {
    StageError::WrongKind {
        stage: "probe".into(),
        layer: layer.into(),
        expected,
        found,
    }
}

fn not_written(stage: &str, layer: &str) -> StageError {
    StageError::NotWritten {
        stage: stage.into(),
        layer: layer.into(),
    }
}

const STEP: StepInfo = StepInfo {
    step: 3,
    steps: 10,
    dt_yr: 25.0,
};

/// Runs `f` in a child context of `process` inside a `probe` context.
fn in_child<R>(
    h: &mut Harness,
    process: &dyn Stage,
    key: &str,
    f: impl FnOnce(&mut StageContext<'_>) -> R,
) -> R {
    let params = process.params(&toml::Table::new()).unwrap();
    h.with(&Probe, |ctx| {
        let mut child = ctx.child(process, params.as_ref(), key, STEP);
        f(&mut child)
    })
}

#[test]
fn reading_an_undeclared_layer_names_stage_and_layer() {
    let mut h = Harness::new();
    let err = h.with(&Probe, |ctx| ctx.f64("biome").unwrap_err());
    assert_eq!(err, undeclared("probe", "biome", Access::Read));
    let text = err.to_string();
    assert!(
        text.contains("`probe`") && text.contains("`biome`"),
        "{text}"
    );
    let err = h.with(&Probe, |ctx| ctx.u8("rivers").unwrap_err());
    assert_eq!(err, undeclared("probe", "rivers", Access::Read));
}

#[test]
fn writing_the_wrong_kind_is_an_error() {
    let mut h = Harness::new();
    let err = h.with(&Probe, |ctx| {
        ctx.write("biome", Layer::F64(Grid::filled(4, 4, 0.0)))
            .unwrap_err()
    });
    assert_eq!(err, wrong_kind("biome", LayerKind::U8, LayerKind::F64));
    let text = err.to_string();
    assert!(
        text.contains("`probe`") && text.contains("`biome`"),
        "{text}"
    );
    assert!(h.world.get("biome").is_none());
}

#[test]
fn writing_an_undeclared_layer_names_it() {
    let mut h = Harness::new();
    let err = h.with(&Probe, |ctx| {
        ctx.write("class", Layer::U8(Grid::filled(4, 4, 9)))
            .unwrap_err()
    });
    assert_eq!(err, undeclared("probe", "class", Access::Write));
    assert_eq!(h.world.u8("class").unwrap().get(0, 0), 2);
}

#[test]
fn a_write_of_the_wrong_shape_names_stage_and_layer() {
    let mut h = Harness::new();
    let err = h.with(&Probe, |ctx| {
        ctx.write("biome", Layer::U8(Grid::filled(3, 3, 0)))
            .unwrap_err()
    });
    assert!(
        matches!(&err, StageError::World { stage, source: WorldError::Shape { name, .. } }
            if stage == "probe" && name == "biome"),
        "{err:?}"
    );
    let text = err.to_string();
    assert!(text.contains("`probe`") && text.contains("biome"), "{text}");
}

#[test]
fn finish_names_a_declared_write_that_was_not_written() {
    let mut h = Harness::new();
    let err = h.with(&Probe, |ctx| {
        ctx.write("height", Layer::F64(Grid::filled(4, 4, 2.0)))
            .unwrap();
        ctx.write("biome", Layer::U8(Grid::filled(4, 4, 1)))
            .unwrap();
        ctx.finish().unwrap_err()
    });
    assert_eq!(err, not_written("probe", "count"));
    let text = err.to_string();
    assert!(
        text.contains("`probe`") && text.contains("`count`"),
        "{text}"
    );
}

#[test]
fn in_place_updates_take_and_write_back() {
    let mut h = Harness::new();
    h.with(&Probe, |ctx| {
        let mut height = ctx.take_f64("height").unwrap();
        height.set(0, 0, 5.0);
        ctx.write("height", Layer::F64(height)).unwrap();
        let mut count = ctx.take_value::<Counter>("count").unwrap();
        count.0 += 1;
        ctx.write("count", Layer::Value(count)).unwrap();
        ctx.write("biome", Layer::U8(Grid::filled(4, 4, 1)))
            .unwrap();
        ctx.finish().unwrap();
        let written: Vec<&str> = ctx.written().iter().map(String::as_str).collect();
        assert_eq!(written, ["biome", "count", "height"]);
    });
    assert!((h.world.f64("height").unwrap().get(0, 0) - 5.0).abs() < 1e-12);
    assert_eq!(h.world.value::<Counter>("count"), Some(&Counter(4)));
}

#[test]
fn a_taken_layer_that_is_not_written_back_fails_finish() {
    let mut h = Harness::new();
    let err = h.with(&Probe, |ctx| {
        ctx.write("height", Layer::F64(Grid::filled(4, 4, 2.0)))
            .unwrap();
        ctx.write("count", Layer::Value(Box::new(Counter(1))))
            .unwrap();
        ctx.write("biome", Layer::U8(Grid::filled(4, 4, 1)))
            .unwrap();
        ctx.take_f64("height").unwrap();
        ctx.finish().unwrap_err()
    });
    assert_eq!(err, not_written("probe", "height"));
}

#[test]
fn optional_writes_may_be_left_out_or_written() {
    let mut h = Harness::new();
    h.with(&Probe, |ctx| {
        ctx.write("height", Layer::F64(Grid::filled(4, 4, 2.0)))
            .unwrap();
        ctx.write("count", Layer::Value(Box::new(Counter(1))))
            .unwrap();
        ctx.write("biome", Layer::U8(Grid::filled(4, 4, 1)))
            .unwrap();
        ctx.finish().unwrap();
        ctx.write("debug", Layer::F64(Grid::filled(4, 4, 0.0)))
            .unwrap();
        ctx.finish().unwrap();
        assert!(ctx.written().contains("debug"));
    });
}

#[test]
fn optional_reads_return_none_when_absent() {
    let mut h = Harness::new();
    h.with(&Probe, |ctx| {
        assert!(ctx.f64_opt("omega").unwrap().is_none());
        assert!(ctx.value_opt::<Label>("tag").unwrap().is_none());
        assert!(ctx.f64_opt("height").unwrap().is_some());
        assert_eq!(
            ctx.value_opt::<Counter>("count").unwrap(),
            Some(&Counter(3))
        );
        assert_eq!(ctx.f64("omega").unwrap_err(), missing("omega"));
    });
    h.world
        .insert("omega", Layer::F64(Grid::filled(4, 4, 0.5)))
        .unwrap();
    h.world
        .insert("tag", Layer::Value(Box::new(Label)))
        .unwrap();
    h.with(&Probe, |ctx| {
        assert!(ctx.f64_opt("omega").unwrap().is_some());
        assert!(ctx.value_opt::<Label>("tag").unwrap().is_some());
    });
}

#[test]
fn every_accessor_returns_its_declared_layer() {
    let mut h = Harness::new();
    h.with(&Probe, |ctx| {
        assert_eq!(ctx.frame().cols, 4);
        assert!((ctx.f64("height").unwrap().get(1, 1) - 1.0).abs() < 1e-12);
        assert_eq!(ctx.u8("class").unwrap().get(0, 0), 2);
        assert!(!ctx.mask("fixed").unwrap().get(0, 0));
        assert_eq!(ctx.u32("receivers").unwrap().get(0, 0), 0);
        assert_eq!(ctx.value::<Counter>("count").unwrap(), &Counter(3));
        assert!(ctx.u8_opt("class").unwrap().is_some());
        assert!(ctx.mask_opt("fixed").unwrap().is_some());
        assert!(ctx.u32_opt("receivers").unwrap().is_some());
    });
}

#[test]
fn reading_with_another_kind_than_declared_is_wrong_kind() {
    let mut h = Harness::new();
    h.with(&Probe, |ctx| {
        assert_eq!(
            ctx.u8("height").unwrap_err(),
            wrong_kind("height", LayerKind::F64, LayerKind::U8)
        );
        assert_eq!(
            ctx.value::<Label>("count").unwrap_err(),
            wrong_kind(
                "count",
                LayerKind::Value("counter"),
                LayerKind::Value("label")
            )
        );
    });
}

#[test]
fn a_layer_of_another_kind_in_the_world_is_wrong_kind() {
    let mut h = Harness::new();
    h.world
        .insert("height", Layer::U8(Grid::filled(4, 4, 0)))
        .unwrap();
    let expected = wrong_kind("height", LayerKind::F64, LayerKind::U8);
    h.with(&Probe, |ctx| {
        assert_eq!(ctx.f64("height").unwrap_err(), expected);
        assert_eq!(ctx.f64_opt("height").unwrap_err(), expected);
        assert_eq!(ctx.take_f64("height").unwrap_err(), expected);
    });
    assert!(
        h.world.u8("height").is_some(),
        "a refused take leaves the layer"
    );
}

#[test]
fn a_value_of_another_type_under_a_declared_name_is_wrong_kind() {
    let mut h = Harness::new();
    h.world
        .insert("count", Layer::Value(Box::new(Label)))
        .unwrap();
    let expected = wrong_kind(
        "count",
        LayerKind::Value("counter"),
        LayerKind::Value("label"),
    );
    h.with(&Probe, |ctx| {
        assert_eq!(ctx.value::<Counter>("count").unwrap_err(), expected);
        assert_eq!(ctx.value_opt::<Counter>("count").unwrap_err(), expected);
        assert_eq!(ctx.take_value::<Counter>("count").unwrap_err(), expected);
    });
    assert!(
        h.world.value::<Label>("count").is_some(),
        "a refused take leaves the layer"
    );
}

#[test]
fn an_absent_required_read_is_missing() {
    let mut h = Harness::new();
    h.world.remove("height");
    h.world.remove("count");
    h.with(&Probe, |ctx| {
        assert_eq!(ctx.f64("height").unwrap_err(), missing("height"));
        assert_eq!(ctx.take_f64("height").unwrap_err(), missing("height"));
        assert_eq!(ctx.value::<Counter>("count").unwrap_err(), missing("count"));
        assert_eq!(
            ctx.take_value::<Counter>("count").unwrap_err(),
            missing("count")
        );
    });
}

#[test]
fn take_needs_the_layer_declared_as_read_and_write() {
    let mut h = Harness::new();
    h.with(&Probe, |ctx| {
        assert_eq!(
            ctx.take_u8("class").unwrap_err(),
            undeclared("probe", "class", Access::Write)
        );
        assert_eq!(
            ctx.take_u8("biome").unwrap_err(),
            undeclared("probe", "biome", Access::Read)
        );
        assert_eq!(
            ctx.take_mask("fixed").unwrap_err(),
            undeclared("probe", "fixed", Access::Write)
        );
        assert_eq!(
            ctx.take_u32("receivers").unwrap_err(),
            undeclared("probe", "receivers", Access::Write)
        );
        assert_eq!(
            ctx.take_u8("height").unwrap_err(),
            wrong_kind("height", LayerKind::F64, LayerKind::U8)
        );
        assert_eq!(
            ctx.take_value::<Label>("tag").unwrap_err(),
            undeclared("probe", "tag", Access::Write)
        );
    });
    assert!(h.world.u8("class").is_some() && h.world.f64("height").is_some());
}

#[test]
fn a_child_context_sees_only_its_process_layers() {
    let mut h = Harness::new();
    let params = Step.params(&toml::Table::new()).unwrap();
    h.with(&Probe, |ctx| {
        {
            let mut child = ctx.child(&Step, params.as_ref(), "probe#0/step#0", STEP);
            assert_eq!(child.stage(), "probe/step");
            assert_eq!(child.key(), "probe#0/step#0");
            assert_eq!(child.step(), Some(STEP));
            assert!(child.f64("height").is_ok());
            assert_eq!(
                child.u8("class").unwrap_err(),
                undeclared("probe/step", "class", Access::Read)
            );
            assert_eq!(
                child
                    .write("height", Layer::F64(Grid::filled(4, 4, 0.0)))
                    .unwrap_err(),
                undeclared("probe/step", "height", Access::Write)
            );
            assert_eq!(
                child.finish().unwrap_err(),
                not_written("probe/step", "biome")
            );
            child
                .write("biome", Layer::U8(Grid::filled(4, 4, 9)))
                .unwrap();
            child.finish().unwrap();
            let mut expected = StageRng::for_key(SEED, "probe#0/step#0");
            assert_eq!(child.rng().next_u64(), expected.next_u64());
        }
        assert_eq!(ctx.step(), None);
        assert!(
            ctx.written().contains("biome"),
            "a child's writes count for its parent"
        );
        assert_eq!(ctx.u8("class").unwrap().get(0, 0), 2);
    });
    assert_eq!(h.world.u8("biome").unwrap().get(0, 0), 9);
}

#[test]
fn writes_of_nested_children_reach_every_ancestor() {
    let mut h = Harness::new();
    let probe = Probe.params(&toml::Table::new()).unwrap();
    let step = Step.params(&toml::Table::new()).unwrap();
    h.with(&Probe, |ctx| {
        {
            let mut child = ctx.child(&Probe, probe.as_ref(), "probe#0/probe#0", STEP);
            {
                let mut grandchild =
                    child.child(&Step, step.as_ref(), "probe#0/probe#0/step#0", STEP);
                assert_eq!(grandchild.stage(), "probe/probe/step");
                grandchild
                    .write("biome", Layer::U8(Grid::filled(4, 4, 1)))
                    .unwrap();
            }
            assert!(child.written().contains("biome"));
        }
        assert!(ctx.written().contains("biome"));
    });
}

#[test]
fn sub_streams_are_named_and_stable() {
    let mut h = Harness::new();
    h.with(&Probe, |ctx| {
        let first = ctx.sub_rng("z0").next_u64();
        assert_eq!(first, StageRng::for_key(SEED, "probe#0/z0").next_u64());
        assert_eq!(
            ctx.sub_rng("z0").next_u64(),
            first,
            "a sub-stream starts afresh at every call"
        );
        assert_ne!(ctx.sub_rng("drift").next_u64(), first);
        let main = ctx.rng().next_u64();
        assert_eq!(main, StageRng::for_key(SEED, "probe#0").next_u64());
        assert_ne!(main, first);
        assert_eq!(
            ctx.sub_rng("z0").next_u64(),
            first,
            "drawing from the stage stream leaves the sub-streams alone"
        );
    });
}

#[test]
fn cancel_and_progress_reach_the_caller() {
    let mut h = Harness::new();
    h.with(&Probe, |ctx| {
        assert!(!ctx.is_cancelled());
        assert!(ctx.check_cancel().is_ok());
        ctx.progress(1, 4);
    });
    h.cancel.cancel();
    h.with(&Probe, |ctx| {
        assert!(ctx.is_cancelled());
        assert_eq!(ctx.check_cancel().unwrap_err(), StageError::Cancelled);
    });
    assert_eq!(
        h.events,
        [Progress {
            stage: "probe".into(),
            done: 1,
            total: 4
        }]
    );
}

#[test]
fn a_child_reports_progress_and_sees_the_cancel() {
    let mut h = Harness::new();
    h.cancel.cancel();
    in_child(&mut h, &Step, "probe#0/step#0", |child| {
        assert!(child.is_cancelled());
        child.progress(2, 5);
    });
    assert_eq!(
        h.events,
        [Progress {
            stage: "probe/step".into(),
            done: 2,
            total: 5
        }]
    );
}

#[test]
fn fail_names_the_stage() {
    let mut h = Harness::new();
    h.with(&Probe, |ctx| {
        assert_eq!(ctx.stage(), "probe");
        assert_eq!(ctx.key(), "probe#0");
        assert_eq!(ctx.step(), None);
        let err = ctx.fail("no land cells");
        assert_eq!(
            err,
            StageError::Failed {
                stage: "probe".into(),
                message: "no land cells".into()
            }
        );
        assert_eq!(err.to_string(), "stage `probe` failed: no land cells");
    });
}

#[test]
fn every_raster_kind_can_be_updated_in_place() {
    let mut h = Harness::new();
    h.with(&Update, |ctx| {
        let mut class = ctx.take_u8("class").unwrap();
        let mut fixed = ctx.take_mask("fixed").unwrap();
        let mut receivers = ctx.take_u32("receivers").unwrap();
        assert!(
            ctx.u8_opt("class").unwrap().is_none(),
            "a taken layer is out of the world"
        );
        class.set(0, 0, 7);
        fixed.set(0, 0, true);
        receivers.set(0, 0, 5);
        ctx.write("class", Layer::U8(class)).unwrap();
        ctx.write("fixed", Layer::Mask(fixed)).unwrap();
        ctx.write("receivers", Layer::U32(receivers)).unwrap();
        ctx.finish().unwrap();
    });
    assert_eq!(h.world.u8("class").unwrap().get(0, 0), 7);
    assert!(h.world.mask("fixed").unwrap().get(0, 0));
    assert_eq!(h.world.u32("receivers").unwrap().get(0, 0), 5);
}

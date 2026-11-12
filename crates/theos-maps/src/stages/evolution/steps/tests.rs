use crate::progress::CancelToken;
use crate::rng::StageRng;
use crate::stage::{Severity, StageError, StageReport};
use crate::stages::evolution::test_support::{
    Draws, SEED, evolve_with, grid_world, logged, processes_of, run_evolution, test_evolution,
};
use crate::world::World;

/// The progress events of a run as `(stage, done, total)`.
type Events = Vec<(String, u32, u32)>;

/// Runs the test evolution with `toml` over a grid world; returns the world, the result and the
/// progress events.
fn run(toml: &str, cancel: &CancelToken) -> (World, Result<StageReport, StageError>, Events) {
    let mut world = grid_world();
    let mut events = Vec::new();
    let result = run_evolution(&mut world, &test_evolution(), toml, cancel, &mut events);
    let events = events
        .into_iter()
        .map(|p| (p.stage, p.done, p.total))
        .collect();
    (world, result, events)
}

#[test]
fn progress_is_reported_every_ten_steps() {
    let (_, result, events) = run("steps = 25\n[[process]]\nid = \"log\"", &CancelToken::new());
    result.unwrap();
    let stage = "landscape_evolution".to_owned();
    assert_eq!(
        events,
        [
            (stage.clone(), 0, 25),
            (stage.clone(), 10, 25),
            (stage, 20, 25)
        ]
    );
}

#[test]
fn cancel_is_checked_every_step() {
    let cancel = CancelToken::new();
    let token = cancel.clone();
    let mut world = grid_world();
    let mut steps_seen = Vec::new();
    let result = {
        let stage = test_evolution();
        let stage: &dyn crate::stage::Stage = &stage;
        let params = stage
            .params(&toml::from_str("steps = 40\n[[process]]\nid = \"log\"").unwrap())
            .unwrap();
        let mut sink = |p: &crate::progress::Progress| {
            steps_seen.push(p.done);
            if p.done == 10 {
                token.cancel();
            }
        };
        let mut ctx = crate::stage::StageContext::new(
            &mut world,
            stage,
            params.as_ref(),
            "landscape_evolution#0",
            3,
            &cancel,
            &mut sink,
        );
        stage.run(&mut ctx, params.as_ref())
    };
    assert_eq!(result, Err(StageError::Cancelled));
    assert_eq!(steps_seen, [0, 10]);
    assert_eq!(
        logged(&world),
        (0..=10).collect::<Vec<u32>>(),
        "cancelled at step 10's progress event, stopped before step 11"
    );

    let cancelled = CancelToken::new();
    cancelled.cancel();
    let (world, result, events) = run("steps = 5\n[[process]]\nid = \"log\"", &cancelled);
    assert_eq!(result, Err(StageError::Cancelled));
    assert!(events.is_empty(), "{events:?}");
    assert!(world.get("log").is_none(), "step 0 never ran");
}

#[test]
fn a_process_with_every_runs_at_step_zero_multiples_and_the_last_step() {
    let (world, result, _) = run(
        "steps = 8\n[[process]]\nid = \"log\"\nevery = 3",
        &CancelToken::new(),
    );
    result.unwrap();
    assert_eq!(logged(&world), [0, 3, 6, 7]);
    let (world, result, _) = run(
        "steps = 7\n[[process]]\nid = \"log\"\nevery = 3",
        &CancelToken::new(),
    );
    result.unwrap();
    assert_eq!(logged(&world), [0, 3, 6], "the last step is a multiple");
    let (world, result, _) = run("steps = 4\n[[process]]\nid = \"log\"", &CancelToken::new());
    result.unwrap();
    assert_eq!(logged(&world), [0, 1, 2, 3], "without `every`, every step");
}

#[test]
fn checks_run_every_check_every_steps_and_on_the_last() {
    let (_, result, _) = run(
        "steps = 10\ncheck_every = 4\n[[process]]\nid = \"judge\"",
        &CancelToken::new(),
    );
    let report = result.unwrap();
    assert_eq!(report.findings.len(), 1, "{:?}", report.findings);
    let warning = &report.findings[0];
    assert_eq!(warning.severity, Severity::Warning);
    assert_eq!(
        warning.message,
        "process `judge` at step 0: checked at step 0 (3 more warnings from this process)",
        "checks at steps 0, 4, 8 and 9"
    );

    let (_, result, _) = run(
        "steps = 10\ncheck_every = 4\n[[process]]\nid = \"judge\"\nfail_at = 9",
        &CancelToken::new(),
    );
    match result {
        Err(StageError::Failed { stage, message }) => {
            assert_eq!(stage, "landscape_evolution/judge");
            assert!(
                message.contains("step 9") && message.contains("judged"),
                "{message}"
            );
        }
        other => panic!("{other:?}"),
    }

    let (_, result, _) = run(
        "steps = 10\ncheck_every = 4\n[[process]]\nid = \"judge\"\nfail_at = 5",
        &CancelToken::new(),
    );
    assert!(result.is_ok(), "step 5 is not a check step: {result:?}");
}

#[test]
fn warnings_of_a_run_are_folded_per_process_every_step() {
    let toml = "steps = 3\ncheck_every = 100\n\
                [[process]]\nid = \"judge\"\nwarn_in_run = true\n\
                [[process]]\nid = \"judge\"";
    let (_, result, _) = run(toml, &CancelToken::new());
    let findings = result.unwrap().findings;
    let messages: Vec<&str> = findings.iter().map(|f| f.message.as_str()).collect();
    assert_eq!(
        messages,
        [
            "process `judge` at step 0: ran at step 0 (4 more warnings from this process)",
            "process `judge#1` at step 0: checked at step 0 (1 more warning from this process)",
        ]
    );
    assert_eq!(findings[0].layer.as_deref(), Some("height"));
}

#[test]
fn a_process_that_skips_a_declared_write_fails_naming_it() {
    let (_, result, _) = run("steps = 2\n[[process]]\nid = \"lazy\"", &CancelToken::new());
    assert_eq!(
        result,
        Err(StageError::NotWritten {
            stage: "landscape_evolution/lazy".to_owned(),
            layer: "receivers".to_owned(),
        })
    );
}

#[test]
fn processes_run_in_order_on_the_initial_surface() {
    let mut start = grid_world();
    let (world, result, _) = run(
        "steps = 2\n[[process]]\nid = \"scale\"\nfactor = 2.0",
        &CancelToken::new(),
    );
    result.unwrap();
    let mut events = Vec::new();
    run_evolution(
        &mut start,
        &test_evolution(),
        "steps = 1",
        &CancelToken::new(),
        &mut events,
    )
    .unwrap();
    let initial = start.f64("height").unwrap().as_slice();
    let scaled = world.f64("height").unwrap().as_slice();
    for (a, b) in initial.iter().zip(scaled) {
        assert_eq!((a * 4.0).to_bits(), b.to_bits());
    }
    for layer in ["erosion", "deposition"] {
        let zeros = world.f64(layer).unwrap().as_slice().iter();
        assert!(zeros.copied().all(|v| v.to_bits() == 0), "{layer}");
    }
}

/// The initial height of the test runs (one step, no processes).
fn initial_height() -> Vec<f64> {
    let (world, result, _) = run("steps = 1", &CancelToken::new());
    result.unwrap();
    world.f64("height").unwrap().as_slice().to_vec()
}

#[test]
fn processes_run_in_list_order() {
    let initial = initial_height();
    let doubled_then_bumped = "steps = 1\n[[process]]\nid = \"scale\"\nfactor = 2.0\n\
                               [[process]]\nid = \"bump\"";
    let bumped_then_doubled = "steps = 1\n[[process]]\nid = \"bump\"\n\
                               [[process]]\nid = \"scale\"\nfactor = 2.0";
    for (toml, expected) in [
        (
            doubled_then_bumped,
            initial.iter().map(|h| h * 2.0 + 1.0).collect::<Vec<_>>(),
        ),
        (
            bumped_then_doubled,
            initial.iter().map(|h| (h + 1.0) * 2.0).collect(),
        ),
    ] {
        let (world, result, _) = run(toml, &CancelToken::new());
        result.unwrap();
        let height = world.f64("height").unwrap().as_slice();
        for (h, e) in height.iter().zip(&expected) {
            assert_eq!(h.to_bits(), e.to_bits(), "{toml}");
        }
    }
}

#[test]
fn process_values_are_summed_per_process_over_the_run() {
    let toml = "steps = 5\n[[process]]\nid = \"judge\"\n\
                [[process]]\nid = \"judge\"\nevery = 2";
    let (_, result, _) = run(toml, &CancelToken::new());
    let values = result.unwrap().values;
    let entries: Vec<(&str, f64)> = values.iter().map(|(k, v)| (k.as_str(), *v)).collect();
    assert_eq!(entries.len(), 2, "{entries:?}");
    assert_eq!(entries[0].0, "judge_1_step");
    assert!(
        (entries[0].1 - 6.0).abs() < 1e-12,
        "steps 0 + 2 + 4: {entries:?}"
    );
    assert_eq!(entries[1].0, "judge_step");
    assert!(
        (entries[1].1 - 10.0).abs() < 1e-12,
        "steps 0 to 4: {entries:?}"
    );
}

#[test]
fn evolve_returns_summed_values_and_folded_warnings() {
    let stage = test_evolution();
    let planned = processes_of(&stage, "[[process]]\nid = \"judge\"\nwarn_in_run = true");
    let mut world = grid_world();
    let report = evolve_with(&stage, &mut world, &planned, 4, 1.0, SEED).unwrap();
    assert!(
        (report.values["judge_step"] - 6.0).abs() < 1e-12,
        "{report:?}"
    );
    let messages: Vec<&str> = report.findings.iter().map(|f| f.message.as_str()).collect();
    assert_eq!(
        messages,
        ["process `judge` at step 0: ran at step 0 (7 more warnings from this process)"],
        "a run warning and a check warning on each of 4 steps"
    );
}

#[test]
fn each_process_keeps_one_random_stream_for_the_run() {
    let stage = test_evolution();
    let planned = processes_of(
        &stage,
        "[[process]]\nid = \"draw\"\n[[process]]\nid = \"log\"",
    );
    let draws = || {
        let mut world = grid_world();
        evolve_with(&stage, &mut world, &planned, 3, 1.0, SEED).unwrap();
        world.value::<Draws>("draws").unwrap().0.clone()
    };
    let first = draws();
    let mut stream = StageRng::for_key(SEED, "landscape_evolution#0/draw#0");
    let expected: Vec<u64> = (0..3).map(|_| stream.next_u64()).collect();
    assert_eq!(
        first, expected,
        "one stream, keyed once per run (plan ruling L12)"
    );
    assert!(first[0] != first[1] && first[1] != first[2]);
    assert_eq!(draws(), first, "the same sequence on a rerun");
}

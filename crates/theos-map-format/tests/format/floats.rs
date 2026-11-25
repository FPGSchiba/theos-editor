//! Numbers in a manifest survive write, read and write unchanged.

use std::io::Cursor;

use serde_json::json;
use theos_map_format::{LayerData, LayerKey, read, write};

use crate::writer::fixture;

// AC: CE-76.1
#[test]
fn awkward_floats_survive_a_round_trip_byte_for_byte() -> Result<(), Box<dyn std::error::Error>> {
    let mut package = fixture()?;
    let scale = 1.0 / 255.0;
    let key = LayerKey {
        level: None,
        name: "water".to_owned(),
    };
    let len = package.layers[&key].len();
    let mut raw = vec![0_u8; len];
    raw[0] = 231;
    package.layers.insert(key, LayerData::U8(raw));
    let entry = package
        .manifest
        .layers
        .iter_mut()
        .find(|l| l.name == "water")
        .ok_or("water")?;
    entry.scale = Some(scale);
    let steps: Vec<f64> = (0..=255).map(|k| f64::from(k) * scale).collect();
    package.manifest.summary = json!({"sum": 0.1 + 0.2, "third": 1.0 / 3.0, "steps": steps});
    package.manifest.frame.cell_m = 0.1 + 0.2;

    let first = write(&package, Cursor::new(Vec::new()))?.into_inner();
    let back = read(Cursor::new(first.clone()))?;
    let second = write(&back, Cursor::new(Vec::new()))?.into_inner();
    assert_eq!(first, second);
    assert_eq!(
        back.manifest.frame.cell_m.to_bits(),
        (0.1_f64 + 0.2).to_bits()
    );
    Ok(())
}

//! The golden record of a run: the SHA-256 of every layer each stage wrote (CE-77).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The hashes of one run, as stored in a golden file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldenRun {
    /// The recipe id.
    pub recipe: String,
    /// The run seed.
    pub seed: u64,
    /// The feature set the generator was built with.
    pub features: String,
    /// The hash of the written package.
    pub package: String,
    /// The stages in pipeline order.
    #[serde(rename = "stage")]
    pub stages: Vec<GoldenStage>,
}

/// The layers one stage wrote, by name, with their SHA-256 in hex.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldenStage {
    /// The stage key, `<id>#<occurrence>`.
    pub key: String,
    /// Layer name to hash.
    pub layers: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stages_serialize_as_stage_tables() {
        let run = GoldenRun {
            recipe: "r".to_owned(),
            seed: 1,
            features: String::new(),
            package: String::new(),
            stages: vec![GoldenStage {
                key: "a#0".to_owned(),
                layers: BTreeMap::from([("x".to_owned(), "ab".to_owned())]),
            }],
        };
        let text = toml::to_string(&run).unwrap();
        assert!(text.contains("[[stage]]"), "{text}");
        let back: GoldenRun = toml::from_str(&text).unwrap();
        assert_eq!(back, run);
    }
}

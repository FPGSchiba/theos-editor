//! Value checks over places: floor numbers and parent cycles (spec §5.10).

use theos_content::{Index, Project, parts};

use crate::common::{TempProject, expected, fixture, keys};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const LOW: &str = "world/synthetic-low.md";
const FLOOR_4: &str = "world/synthetic-low/f04-synthetic.md";
const FLOOR_25: &str = "world/synthetic-low/f25-synthetic.md";

fn indexed(project: &Project) -> Result<Index, Box<dyn std::error::Error>> {
    let mut index = Index::in_memory(project, &parts::builtin()?)?;
    index.sync()?;
    Ok(index)
}

// AC: CE-14.2
#[test]
fn world_findings_equal_the_expectation() -> TestResult {
    let index = indexed(&Project::open(&fixture("world"))?)?;
    assert_eq!(
        keys(&index.findings()?),
        expected(&fixture("world").join("expected.toml"))?
    );
    Ok(())
}

// AC: CE-14.3
#[test]
fn changing_a_band_revalidates_its_floors() -> TestResult {
    let temp = TempProject::copy_of("world")?;
    let mut index = indexed(&temp.project()?)?;
    let floors = |index: &Index| -> Result<Vec<String>, Box<dyn std::error::Error>> {
        Ok(index
            .findings()?
            .into_iter()
            .filter(|f| f.rule == "world/floor-number")
            .map(|f| f.path)
            .collect())
    };
    assert_eq!(floors(&index)?, [FLOOR_25]);

    let band = std::fs::read_to_string(temp.root().join(LOW))?;
    assert!(band.contains("floors: [1, 20]"));
    temp.write(LOW, &band.replace("floors: [1, 20]", "floors: [10, 20]"))?;
    let update = index.sync()?;
    assert_eq!(update.reparsed, [LOW]);
    for floor in [FLOOR_4, FLOOR_25] {
        assert!(
            update.revalidated.iter().any(|path| path == floor),
            "{floor} is revalidated: {:?}",
            update.revalidated
        );
    }
    assert_eq!(floors(&index)?, [FLOOR_4, FLOOR_25]);
    let cold = indexed(&temp.project()?)?;
    assert_eq!(index.dump()?, cold.dump()?);
    Ok(())
}

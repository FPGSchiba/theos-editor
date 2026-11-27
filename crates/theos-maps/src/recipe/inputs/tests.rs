use super::*;

fn decls(toml_text: &str) -> Inputs {
    let table: BTreeMap<String, InputDecl> = toml::from_str(toml_text).unwrap();
    Inputs(table)
}

fn given(json: &Value) -> Map<String, Value> {
    json.as_object().unwrap().clone()
}

#[test]
fn describe_reports_a_feature_input() {
    let inputs = decls("feature = { type = \"feature\", parent_types = [\"town\", \"city\"] }");
    assert_eq!(
        Value::Object(inputs.describe()),
        serde_json::json!({"feature": {"type": "feature", "parent_types": ["town", "city"]}})
    );
    assert_eq!(inputs.feature().unwrap().0, "feature");
}

#[test]
fn declarations_are_validated() {
    for bad in [
        "a = { type = \"feature\", parent_types = [\"t\"] }\nb = { type = \"feature\", parent_types = [\"t\"] }",
        "a = { type = \"feature\", parent_types = [] }",
        "a = { type = \"number\", min = 2, max = 1 }",
        "a = { type = \"enum\", values = [] }",
        "a = { type = \"enum\", values = [\"x\"], default = \"y\" }",
    ] {
        assert!(decls(bad).validate().is_err(), "{bad}");
    }
    assert!(
        decls("a = { type = \"number\", min = 1, max = 1 }")
            .validate()
            .is_ok()
    );
}

#[test]
fn enum_values_are_checked() {
    let inputs = decls("size = { type = \"enum\", values = [\"small\", \"big\"] }");
    assert!(
        inputs
            .check(&given(&serde_json::json!({"size": "big"})))
            .is_ok()
    );
    assert!(
        inputs
            .check(&given(&serde_json::json!({"size": "huge"})))
            .is_err()
    );
}

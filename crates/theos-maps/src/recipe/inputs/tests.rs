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
    let two = "a = { type = \"feature\", parent_types = [\"t\"] }\n\
               b = { type = \"feature\", parent_types = [\"t\"] }";
    for (bad, message) in [
        (two, "only one `feature` input"),
        (
            "a = { type = \"feature\", parent_types = [] }",
            "needs `parent_types`",
        ),
        (
            "a = { type = \"number\", min = 2, max = 1 }",
            "`min` 2 is above `max` 1",
        ),
        ("a = { type = \"enum\", values = [] }", "needs `values`"),
        (
            "a = { type = \"enum\", values = [\"x\"], default = \"y\" }",
            "`default` `y` is not one of the values",
        ),
        (
            "a = { type = \"number\", min = 1, max = 5, default = 9 }",
            "outside its range",
        ),
        (
            "a = { type = \"number\", integer = true, default = 1.5 }",
            "whole number",
        ),
    ] {
        let problem = decls(bad).validate().unwrap_err();
        assert!(
            problem.message.contains(message),
            "{bad}: {}",
            problem.message
        );
    }
    assert!(
        decls("a = { type = \"number\", min = 1, max = 1, default = 1 }")
            .validate()
            .is_ok()
    );
}

#[test]
fn absent_options_are_not_reported() {
    let inputs = decls("n = { type = \"number\", min = 1 }");
    assert_eq!(
        Value::Object(inputs.describe()),
        serde_json::json!({"n": {"type": "number", "min": 1.0, "integer": false}})
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

use super::AddConversionFn;
use crate::converter::api::Language;

pub fn add_json(input: &str, add: &mut impl AddConversionFn) -> bool {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&input) else {
        return false;
    };
    if let Ok(json) = serde_json::to_string_pretty(&json) {
        add(Language::new("json"), json);
    }
    if let Ok(yaml) = serde_yaml_ng::to_string(&json) {
        add(Language::new("yaml"), yaml);
    }
    return true;
}

pub fn add_yaml(input: &str, add: &mut impl AddConversionFn) {
    let Ok(json) = serde_yaml_ng::from_str::<serde_json::Value>(&input) else {
        return;
    };
    if let Ok(json) = serde_json::to_string_pretty(&json) {
        add(Language::new("json"), json);
    }
    if let Ok(yaml) = serde_yaml_ng::to_string(&json) {
        add(Language::new("yaml"), yaml);
    }
}

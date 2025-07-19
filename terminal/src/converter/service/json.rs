use super::AddConversionFn;
use crate::converter::api::Language;

pub fn add_json(input: &str, add: &mut impl AddConversionFn) -> bool {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(input) else {
        return false;
    };
    if let Ok(json) = serde_json::to_string_pretty(&json) {
        add(Language::new("JSON"), json);
    }
    if let Ok(yaml) = serde_yaml_ng::to_string(&json) {
        add(Language::new("YAML"), yaml);
    }
    return true;
}

pub fn add_yaml(input: &str, add: &mut impl AddConversionFn) {
    let Ok(json) = serde_yaml_ng::from_str::<serde_json::Value>(input) else {
        return;
    };
    if let Ok(json) = serde_json::to_string_pretty(&json) {
        add(Language::new("JSON"), json);
    }
    if let Ok(yaml) = serde_yaml_ng::to_string(&json) {
        add(Language::new("YAML"), yaml);
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::GetConversionForTest as _;

    #[tokio::test]
    async fn json_to_json() {
        let conversion =
            r#" { "a": [1,2,3], "b": {"b1":[11],"b2":"22"}} "#.get_conversion("JSON").await;
        assert_eq!(
            r#"{
  "a": [
    1,
    2,
    3
  ],
  "b": {
    "b1": [
      11
    ],
    "b2": "22"
  }
}"#,
            conversion
        );
    }

    #[tokio::test]
    async fn json_to_yaml() {
        let conversion =
            r#" { "a": [1,2,3], "b": {"b1":[11],"b2":"22"}} "#.get_conversion("YAML").await;
        assert_eq!(
            r#"a:
- 1
- 2
- 3
b:
  b1:
  - 11
  b2: '22'
"#,
            conversion
        );
    }

    #[tokio::test]
    async fn yaml_to_json() {
        let conversion = r#"
a:
- 1
- 2
- 3
b:
  b1:
  - 11
  b2: '22'
"#
        .get_conversion("JSON")
        .await;
        assert_eq!(
            r#"{
  "a": [
    1,
    2,
    3
  ],
  "b": {
    "b1": [
      11
    ],
    "b2": "22"
  }
}"#,
            conversion
        );
    }

    #[tokio::test]
    async fn yaml_to_yaml() {
        let conversion = r#"
a:
    - 1
    - 2
    - 3
b:
    b1:
        - 11
    b2: '22'
"#
        .get_conversion("YAML")
        .await;
        assert_eq!(
            r#"a:
- 1
- 2
- 3
b:
  b1:
  - 11
  b2: '22'
"#,
            conversion
        );
    }
}

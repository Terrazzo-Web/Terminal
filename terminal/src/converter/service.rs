#![cfg(feature = "server")]

use nameth::nameth;
use terrazzo::declare_trait_aliias;
use tonic::Status;

use super::api::Conversion;
use super::api::Conversions;
use super::api::ConversionsRequest;
use crate::backend::client_service::remote_fn_service;
use crate::converter::api::Language;

#[nameth]
pub async fn get_conversions(input: String) -> Result<Conversions, Status> {
    let mut conversions = vec![];

    let mut add_conversion = |language, content| {
        conversions.push(Conversion::new(language, content));
    };
    add_json(&input, &mut add_conversion);
    add_yaml(&input, &mut add_conversion);
    return Ok(Conversions {
        conversions: conversions.into(),
    });
}

declare_trait_aliias!(AddConversionFn, FnMut(Language, String));

fn add_json(input: &str, add: &mut impl AddConversionFn) {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&input) {
        if let Ok(json) = serde_json::to_string_pretty(&json) {
            add(Language::new("json"), json);
        }
        if let Ok(yaml) = serde_yaml_ng::to_string(&json) {
            add(Language::new("yaml"), yaml);
        }
    }
}

fn add_yaml(input: &str, add: &mut impl AddConversionFn) {
    if let Ok(json) = serde_yaml_ng::from_str::<serde_json::Value>(&input) {
        if let Ok(json) = serde_json::to_string_pretty(&json) {
            add(Language::new("json"), json);
        }
        if let Ok(yaml) = serde_yaml_ng::to_string(&json) {
            add(Language::new("yaml"), yaml);
        }
    }
}

remote_fn_service::declare_remote_fn!(
    GET_CONVERSIONS_FN,
    GET_CONVERSIONS,
    ConversionsRequest,
    Conversions,
    |_server, arg| get_conversions(arg.input)
);

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn json_to_json() {
        let conversion =
            r#" { "a": [1,2,3], "b": {"b1":[11],"b2":"22"}} "#.get_conversion("json").await;
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
            r#" { "a": [1,2,3], "b": {"b1":[11],"b2":"22"}} "#.get_conversion("yaml").await;
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
        let conversion = r#"a:
- 1
- 2
- 3
b:
  b1:
  - 11
  b2: '22'
"#
        .get_conversion("json")
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
        .get_conversion("yaml")
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

    trait GetConversionForTest {
        async fn get_conversion(&self, language: &str) -> String;
    }

    impl GetConversionForTest for &str {
        async fn get_conversion(&self, language: &str) -> String {
            let conversions = super::get_conversions(self.to_string()).await.unwrap();
            for conversion in conversions.conversions.iter() {
                if conversion.language.name.as_ref() == language {
                    return conversion.content.clone();
                }
            }
            return "Not found".to_string();
        }
    }
}

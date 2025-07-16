#![cfg(feature = "server")]

use nameth::nameth;
use terrazzo::declare_trait_aliias;
use tonic::Status;

use super::api::Conversion;
use super::api::Conversions;
use super::api::ConversionsRequest;
use crate::backend::client_service::remote_fn_service;
use crate::converter::api::Language;

mod json;
mod jwt;

#[nameth]
pub async fn get_conversions(input: String) -> Result<Conversions, Status> {
    let mut conversions = vec![];

    let mut add_conversion = |language, content| {
        conversions.push(Conversion::new(language, content));
    };
    if !self::json::add_json(&input, &mut add_conversion) {
        self::json::add_yaml(&input, &mut add_conversion);
    }
    self::jwt::add_jwt(&input, &mut add_conversion);
    return Ok(Conversions {
        conversions: conversions.into(),
    });
}

declare_trait_aliias!(AddConversionFn, FnMut(Language, String));

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

    #[tokio::test]
    async fn jwt() {
        let conversion = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJleHAiOjE3NTI2ODYyNDAsIm5iZiI6MTc1MjY4NTg4MH0.voEB1O4AnPdCWHARf_1jTNA5CpayxWGyXfMf6p_wfbw"
        .get_conversion("jwt")
        .await;
        assert_eq!(
            r#"
header:
  alg: HS256
  typ: JWT
message:
  exp: 1752686240 = 2025-07-16T17:17:20Z (in 5m 55s)
  nbf: 1752685880 = 2025-07-16T17:11:20Z (5s ago)
signature: voEB1O4AnPdCWHARf_1jTNA5CpayxWGyXfMf6p_wfbw"#
                .trim(),
            conversion.trim()
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

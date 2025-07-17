#![cfg(feature = "server")]

use nameth::nameth;
use terrazzo::declare_trait_aliias;
use tonic::Status;

use super::api::Conversion;
use super::api::Conversions;
use super::api::ConversionsRequest;
use crate::backend::client_service::remote_fn_service;
use crate::converter::api::Language;

mod asn1;
mod json;
mod jwt;
mod x509;

#[nameth]
pub async fn get_conversions(input: String) -> Result<Conversions, Status> {
    let mut conversions = vec![];
    let mut add_conversion = |language, content| {
        conversions.push(Conversion::new(language, content));
    };
    add_conversions(&input, &mut add_conversion);
    return Ok(Conversions {
        conversions: conversions.into(),
    });
}

fn add_conversions(input: &str, add: &mut impl AddConversionFn) {
    if self::x509::add_x509(input, add) {
        return;
    }
    if self::jwt::add_jwt(input, add) {
        return;
    }
    if self::asn1::add_asn1(input, add) {
        return;
    }
    if self::json::add_json(input, add) {
        return;
    }
    self::json::add_yaml(input, add);
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

    pub trait GetConversionForTest {
        async fn get_conversion(&self, language: &str) -> String;
    }

    impl GetConversionForTest for &str {
        async fn get_conversion(&self, language: &str) -> String {
            let conversions = super::get_conversions(self.to_string()).await.unwrap();
            let matches = conversions
                .conversions
                .iter()
                .filter(|conversion| conversion.language.name.as_ref() == language)
                .collect::<Vec<_>>();
            match matches.as_slice() {
                &[] => "Not found".to_string(),
                &[conversion] => conversion.content.clone(),
                _ => "Duplicates".to_string(),
            }
        }
    }
}

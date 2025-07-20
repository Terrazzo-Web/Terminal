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
mod base64;
mod json;
mod jwt;
mod pkcs7;
mod unescaped;
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
    if self::x509::add_x509_pem(input, add) {
        return;
    }
    if self::jwt::add_jwt(input, add) {
        return;
    }
    if self::base64::add_base64(input, add) {
        return;
    }
    if !self::json::add_json(input, add) {
        self::json::add_yaml(input, add);
    }
    self::unescaped::add_unescape(input, add);
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
        async fn get_languages(&self) -> Vec<String>;
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

        async fn get_languages(&self) -> Vec<String> {
            let conversions = super::get_conversions(self.to_string()).await.unwrap();
            let mut languages = conversions
                .conversions
                .iter()
                .map(|conversion| conversion.language.name.to_string())
                .collect::<Vec<_>>();
            languages.sort();
            return languages;
        }
    }
}

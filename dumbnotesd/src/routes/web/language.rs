use std::{borrow::Cow, collections::HashSet, sync::LazyLock};

use async_trait::async_trait;
use language_tags::LanguageTag;
use rocket::{Request, outcome::try_outcome, request::{FromRequest, Outcome}};
use rocket::http::hyper::header;
use rust_i18n::available_locales;

pub static SUPPORTED_LANGUAGES: LazyLock<HashSet<String>> = LazyLock::new(||
    available_locales!()
        .into_iter()
        .map(Cow::into_owned)
        .collect()
);

// TODO: validation
const DEFAULT_LOCALE_NAME: &str = "en";

#[derive(Debug)]
pub struct AcceptedLanguages(Box<[LanguageTag]>);

#[async_trait]
impl<'r> FromRequest<'r> for AcceptedLanguages {
    type Error = ();

    async fn from_request(
        request: &'r Request<'_>,
    ) -> Outcome<Self, Self::Error> {
        let languages_raw = request.headers()
            .get(header::ACCEPT_LANGUAGE.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        let filtered = accept_language::parse(&languages_raw)
            .into_iter()
            .filter_map(|l|
                LanguageTag::parse(&l).ok()
            )
            .filter(|l|
                SUPPORTED_LANGUAGES.contains(l.primary_language())
            )
            .collect();

        Outcome::Success(
            AcceptedLanguages(filtered)
        )
    }
}

#[derive(Debug)]
pub struct BestLanguage(LanguageTag);

#[async_trait]
impl<'r> FromRequest<'r> for BestLanguage {
    type Error = ();

    async fn from_request(
        request: &'r Request<'_>,
    ) -> Outcome<Self, Self::Error> {
        let list = try_outcome!(request.guard::<AcceptedLanguages>().await).0;
        Outcome::Success(
            BestLanguage(
                list.into_iter()
                    .next()
                    .unwrap_or_else(||
                        LanguageTag::parse(&DEFAULT_LOCALE_NAME)
                            .expect("failed to find the default locale \"{DEFAULT_LOCALE_NAME}\"")
                    )
            )
        )
    }
}

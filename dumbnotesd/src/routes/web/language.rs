use std::{collections::HashMap, sync::LazyLock};

use async_trait::async_trait;
use language_tags::LanguageTag;
use rocket::{Request, outcome::try_outcome, request::{FromRequest, Outcome}};
use rocket::http::hyper::header;
use rust_i18n::available_locales;
use smallvec::SmallVec;

pub static SUPPORTED_LANGUAGES: LazyLock<HashMap<String, LanguageTag>> = LazyLock::new(||
    available_locales!()
        .into_iter()
        .map(|l| {
            let l = l.into_owned();
            let tag = LanguageTag::parse(&l)
                .unwrap_or_else(|e|
                    panic!(
                        "failed to parse language tag of supported locale \"{}\": {e}",
                        &l,
                    )
                );
            (l, tag)
        })
        .collect()
);

// TODO: validation
static DEFAULT_LOCALE_NAME: &str = "en";

#[derive(Debug)]
pub struct AcceptedLanguages(pub SmallVec<[&'static LanguageTag; 4]>);

#[async_trait]
impl<'r> FromRequest<'r> for AcceptedLanguages {
    type Error = ();

    async fn from_request(
        request: &'r Request<'_>,
    ) -> Outcome<Self, Self::Error> {
        let languages_raw = request.headers()
            .get(header::ACCEPT_LANGUAGE.as_str())
            .collect::<SmallVec<[_; 4]>>()
            .join(", ");

        let filtered = accept_language::parse(&languages_raw)
            .into_iter()
            .filter_map(|l|
                LanguageTag::parse(&l).ok()
            )
            .filter_map(|l|
                SUPPORTED_LANGUAGES
                    .get(l.primary_language())
            )
            .collect();

        Outcome::Success(
            AcceptedLanguages(filtered)
        )
    }
}

#[derive(Debug)]
pub struct BestLanguage(pub &'static LanguageTag);

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
                        &SUPPORTED_LANGUAGES[DEFAULT_LOCALE_NAME]
                    )
            )
        )
    }
}

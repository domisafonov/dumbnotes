mod static_content;
mod language;
mod translator;
mod authentication_guard;

use askama::Template;
use rocket::response::Redirect;
use rocket::{Build, Rocket, delete, get, post, routes};
use rocket::response::content::RawHtml;
use uuid::Uuid;
use crate::app_constants::WEB_PREFIX;
use crate::routes::web::language::BestLanguage;
use crate::routes::web::static_content::WebStaticContentRocketBuildExt;
use crate::routes::web::translator::{t, Translator};

#[derive(Debug, Template)]
#[template(path = "login.html")]
struct LoginPage {
    t: Translator,
}

// #[get("/login")]
// fn authenticated_login_redirect(
//     _auth: Authenticated,
// ) -> Redirect {
//     Redirect::temporary(format!("{WEB_PREFIX}/login"))
// }

#[get("/login")]
fn login_page(
    language: BestLanguage,
) -> RawHtml<String> {
    RawHtml(
        LoginPage { t: language.0.into() }
            .render()
            .unwrap()
    )
}

#[get("/")]
fn root() -> RawHtml<&'static str> {
    todo!()
}
#[post("/")]
fn login_submit() -> RawHtml<&'static str> {
    todo!()
}
#[get("/notes/<note_id>")]
fn one_note_view(note_id: Uuid) -> RawHtml<&'static str> {
    todo!()
}
#[post("/notes/<note_id>")]
fn save_note(note_id: Uuid, /* TODO */) -> RawHtml<&'static str> {
    todo!()
}
#[delete("/notes/<note_id>")]
fn delete_note(note_id: Uuid) -> RawHtml<&'static str> {
    todo!()
}

pub trait WebRocketBuildExt {
    fn install_dumbnotes_web(self) -> Self;
}

impl WebRocketBuildExt for Rocket<Build> {
    fn install_dumbnotes_web(self) -> Self {
        self
            .install_dumbnotes_web_static_content()
            .mount(
                WEB_PREFIX,
                routes![
                    login_page,
                ]
            )
    }
}

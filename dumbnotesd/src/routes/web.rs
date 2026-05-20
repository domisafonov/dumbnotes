mod language;

use rocket::{Build, Rocket, delete, get, post, routes};
use rocket::response::content::RawHtml;
use rust_i18n::t;
use uuid::Uuid;
use crate::app_constants::WEB_PREFIX;

#[get("/")]
fn web_stub() -> RawHtml<&'static str> {
    RawHtml("<html><head><title>There be web></title></head><body>There be web</body></html>")
}

#[get("/")]
fn root() -> RawHtml<&'static str> {
    t!("a");
    todo!()
}
#[get("/login")]
fn login_page() -> RawHtml<&'static str> {
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
            .mount(
                WEB_PREFIX,
                routes![
                    web_stub,
                ]
            )
    }
}

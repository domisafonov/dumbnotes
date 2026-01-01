use rocket::{get, routes, Build, Rocket};
use rocket::response::content::RawHtml;
use crate::app_constants::WEB_PREFIX;

#[get("/")]
fn web_stub() -> RawHtml<&'static str> {
    RawHtml("<html><head><title>There be web></title></head><body>There be web</body></html>")
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
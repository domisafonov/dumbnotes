use dumbnotesd_web_css::DUMBNOTESD_WEB_CSS;
use rocket::{Build, Responder, Rocket, get, response::content::RawCss, routes};

use crate::app_constants::WEB_PREFIX;

const HTMX_JS: &str = include_str!("htmx-4.0.0-beta4.min.js");

#[derive(Responder)]
#[response(content_type = "js")]
struct RawJs<R>(pub R);

#[get("/js/htmx.js")]
fn get_htmx() -> RawJs<&'static str> {
    RawJs(HTMX_JS)
}

#[get("/css/main.css")]
fn get_css() -> RawCss<&'static str> {
    RawCss(DUMBNOTESD_WEB_CSS)
}

pub trait WebStaticContentRocketBuildExt {
    fn install_dumbnotes_web_static_content(self) -> Self;
}

impl WebStaticContentRocketBuildExt for Rocket<Build> {
    fn install_dumbnotes_web_static_content(self) -> Self {
        self.mount(
            WEB_PREFIX,
            routes![
                get_htmx,
                get_css,
            ]
        )
    }
}

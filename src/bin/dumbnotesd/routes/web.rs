use rocket::{get, routes, Route};
use rocket::response::content::RawHtml;

#[get("/")]
fn web_stub() -> RawHtml<&'static str> {
    RawHtml("<html><head><title>There be web></title></head><body>There be web</body></html>")
}

pub fn web_routes() -> Vec<Route> {
    routes![
        web_stub,
    ]
}

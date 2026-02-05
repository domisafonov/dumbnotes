use std::sync::LazyLock;

pub static RQ: LazyLock<reqwest::blocking::Client> = LazyLock::new(||
    reqwest::blocking::Client::new()
);

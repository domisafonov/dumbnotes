pub mod bindings {
    include!(concat!(env!("OUT_DIR"), "/dumbnotes.auth_ipc.protobuf.rs"));
}

pub mod model {
    pub mod login;
    pub mod refresh_token;
    pub mod logout;
    pub mod successful_login;
}

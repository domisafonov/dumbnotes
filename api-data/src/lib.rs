pub mod bindings {
    include!(concat!(env!("OUT_DIR"), "/dumbnotes.api.protobuf.rs"));
}

mod constants {
    pub const DEFAULT_PROTOBUF_READ_LIMIT: u64 = 1024 * 1024;
}

pub mod model;
mod protobuf;

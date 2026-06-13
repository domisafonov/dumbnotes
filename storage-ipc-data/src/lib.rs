pub mod bindings {
    include!(concat!(env!("OUT_DIR"), "/dumbnotes.storage_ipc.protobuf.rs"));
}

pub mod model {
    pub mod read_note;
    pub mod write_note;
    pub mod list_notes;
    pub mod get_note_details;
    pub mod delete_note;

    mod note_metadata;
    mod note_info;
    mod note;
}

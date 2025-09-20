use std::collections::HashMap;
use std::sync::atomic::Ordering;
use tokio::io;

use lazy_static::lazy_static;
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};
use uuid::Uuid;

use crate::storage::internal::tests::mocks::{FileSpec, VersionedFileSpec};
use crate::storage::internal::rng::{make_uuid, SyncRng};
use crate::storage::internal::TMP_FILENAME_INFIX;

// TODO: write rng seeds when the tests fail
lazy_static!(
    pub static ref RNG: SyncRng<StdRng> = SyncRng::new(StdRng::from_os_rng());
    pub static ref SPEC_RNG: SyncRng<StdRng> = SyncRng::new(StdRng::seed_from_u64(RNG.lock().unwrap().next_u64() + 1));

    // TODO: make strictly sequential
    pub static ref READ_NOTE_NORMAL_UUID: Uuid = make_uuid(&mut SPEC_RNG.lock().unwrap());
    pub static ref READ_NOTE_EMPTY_UUID: Uuid = make_uuid(&mut SPEC_RNG.lock().unwrap());
    pub static ref READ_NOTE_EMPTY_NAME_UUID: Uuid = make_uuid(&mut SPEC_RNG.lock().unwrap());
    pub static ref READ_NOTE_EMPTY_CONTENTS_UUID: Uuid = make_uuid(&mut SPEC_RNG.lock().unwrap());
    pub static ref READ_NOTE_NO_NEWLINE_UUID: Uuid = make_uuid(&mut SPEC_RNG.lock().unwrap());
    pub static ref READ_NOTE_INVALID_UTF8_UUID: Uuid = make_uuid(&mut SPEC_RNG.lock().unwrap());
    pub static ref READ_NOTE_CANT_OPEN_UUID: Uuid = make_uuid(&mut SPEC_RNG.lock().unwrap());
    pub static ref READ_NOTE_CANT_READ_UUID: Uuid = make_uuid(&mut SPEC_RNG.lock().unwrap());

    pub static ref WRITE_NOTE_NORMAL_UUID: Uuid = make_uuid(&mut SPEC_RNG.lock().unwrap());

    pub static ref DEFAULT_SPECS: HashMap<String, VersionedFileSpec> = HashMap::from_iter([
        ("/".into(), FileSpec::Dir),
        ("/meta_fail".into(),
            FileSpec::MetadataError(
                Box::new(|| io::Error::from(io::ErrorKind::StorageFull))
            )
        ),
        ("/a_file".into(), FileSpec::empty_file()),
        ("/no_such_dir".into(),
            FileSpec::MetadataError(
                Box::new(|| io::Error::from(io::ErrorKind::NotFound))
            )
        ),
        ("/not_enough_perms_dir".into(), FileSpec::NotEnoughPermsDir),
        ("/other_owner_dir".into(), FileSpec::OtherOwnerDir),

        ("/read_note".into(), FileSpec::Dir),
        (make_path("/read_note", *READ_NOTE_NORMAL_UUID),
            FileSpec::File {
               contents: "normal title\nnormal contents".as_bytes().into(),
            },
        ),
        (make_path("/read_note", *READ_NOTE_EMPTY_UUID), FileSpec::empty_file()),
        (make_path("/read_note", *READ_NOTE_EMPTY_NAME_UUID),
            FileSpec::File {
                contents: "\nnormal contents".as_bytes().into(),
            }
        ),
        (make_path("/read_note", *READ_NOTE_EMPTY_CONTENTS_UUID),
            FileSpec::File {
                contents: "normal title\n".as_bytes().into(),
            }
        ),
        (make_path("/read_note", *READ_NOTE_NO_NEWLINE_UUID),
            FileSpec::File {
                contents: "normal title".as_bytes().into(),
            }
        ),
        (make_path("/read_note", *READ_NOTE_INVALID_UTF8_UUID),
            FileSpec::File {
                contents: vec!(
                    0xA0, 0xA1,
                    b'o', b'k', b't',
                    0xE2, 0x28, 0xA1, // 0x28 is '('
                    b'\n',
                    0xC3, 0x28, // 0x28 is '('
                    b'o', b'k', b'c',
                    0xFC, 0xA1, 0xA1, 0xA1, 0xA1, 0xA1,
                ),
            }
        ),
        (make_path("/read_note", *READ_NOTE_CANT_OPEN_UUID), FileSpec::CantOpen),
        (make_path("/read_note", *READ_NOTE_CANT_READ_UUID), FileSpec::CantRead),

        ("/write_note".into(), FileSpec::Dir),
        (make_tmp_path("/write_note", *WRITE_NOTE_NORMAL_UUID), FileSpec::WriteTmpFile),
        (make_tmp_path("/write_note", *WRITE_NOTE_NORMAL_UUID),
            FileSpec::RenameWrittenTmpFile {
                path: make_tmp_path("/write_note", *WRITE_NOTE_NORMAL_UUID).path,
                rename_to: make_path("/write_note", *WRITE_NOTE_NORMAL_UUID).path
            }
        ),
    ]
        .into_iter()
        .map(|(DataPath { path, is_tmp },v)|
            (path, VersionedFileSpec::new(v, is_tmp))
        )
        .fold(
            Vec::<(String, VersionedFileSpec)>::new(), 
            |mut r, (k, mut v): (_, VersionedFileSpec)| {
                match r.last_mut() {
                    Some((last_key, last)) if *last_key == k => {
                        last.specs.push(
                            v.specs
                                .drain(..)
                                .next().expect("the singular element of the spec list")
                        );
                        last.current_version.store(0, Ordering::Relaxed);
                    },
                    _ => r.push((k, v)),
                }
                r
            }
        )
    );
);

pub const READ_NOTE_INVALID_UTF8_TITLE: &str = "okt(";
pub const READ_NOTE_INVALID_UTF8_CONTENTS: &str = "(okc";

pub struct DataPath {
    pub path: String,
    pub is_tmp: bool,
}

impl From<&str> for DataPath {
    fn from(s: &str) -> DataPath {
        DataPath {
            path: s.to_string(),
            is_tmp: false,
        }
    }
}

pub fn make_path(base: &str, uuid: Uuid) -> DataPath {
    DataPath { 
        path: base.to_string() + "/" + &uuid.hyphenated().to_string(),
        is_tmp: false
    }
}

pub fn make_tmp_path(base: &str, uuid: Uuid) -> DataPath {
    DataPath {
        path: base.to_string() + "/" + &uuid.hyphenated().to_string() 
            + TMP_FILENAME_INFIX,
        is_tmp: true
    }
}

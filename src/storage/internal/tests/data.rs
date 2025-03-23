use std::collections::HashMap;
use tokio::io;

use lazy_static::lazy_static;
use uuid::Uuid;

use crate::storage::internal::tests::mocks::FileSpec;

lazy_static!(
    pub static ref READ_NOTE_NORMAL_UUID: Uuid = Uuid::new_v4();
    pub static ref READ_NOTE_EMPTY_UUID: Uuid = Uuid::new_v4();
    pub static ref READ_NOTE_EMPTY_NAME_UUID: Uuid = Uuid::new_v4();
    pub static ref READ_NOTE_EMPTY_CONTENTS_UUID: Uuid = Uuid::new_v4();
    pub static ref READ_NOTE_NO_NEWLINE_UUID: Uuid = Uuid::new_v4();
    pub static ref READ_NOTE_INVALID_UTF8: Uuid = Uuid::new_v4();

    pub static ref DEFAULT_SPECS: HashMap<String, FileSpec> = HashMap::from([
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
        ("/note_dir".into(), FileSpec::Dir),
        ("/read_note".into(), FileSpec::Dir),
        ("/read_note/".to_string() + &READ_NOTE_NORMAL_UUID.hyphenated().to_string(),
            FileSpec::File {
               contents: "normal title\nnormal contents".as_bytes().into(),
            },
        ),
        ("/read_note/".to_string() + &READ_NOTE_EMPTY_UUID.hyphenated().to_string(),
            FileSpec::empty_file()
        ),
        ("/read_note/".to_string() + &READ_NOTE_EMPTY_NAME_UUID.hyphenated().to_string(),
            FileSpec::File {
                contents: "\nnormal contents".as_bytes().into(),
            }
        ),
        ("/read_note/".to_string() + &READ_NOTE_EMPTY_CONTENTS_UUID.hyphenated().to_string(),
            FileSpec::File {
                contents: "normal title\n".as_bytes().into(),
            }
        ),
        ("/read_note/".to_string() + &READ_NOTE_NO_NEWLINE_UUID.hyphenated().to_string(),
            FileSpec::File {
                contents: "normal title".as_bytes().into(),
            }
        ),
        ("/read_note/".to_string() + &READ_NOTE_INVALID_UTF8.hyphenated().to_string(),
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
    ]);
);

pub const READ_NOTE_INVALID_UTF8_TITLE: &str = "okt(";
pub const READ_NOTE_INVALID_UTF8_CONTENTS: &str = "(okc";
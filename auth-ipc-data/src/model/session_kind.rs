use data::SessionKind;

use crate::bindings;

impl From<SessionKind> for bindings::SessionKind {
    fn from(value: SessionKind) -> Self {
        match value {
            SessionKind::Api => bindings::SessionKind::Api,
            SessionKind::Web => bindings::SessionKind::Web,
        }
    }
}

impl From<bindings::SessionKind> for SessionKind {
    fn from(value: bindings::SessionKind) -> Self {
        match value {
            bindings::SessionKind::Api => SessionKind::Api,
            bindings::SessionKind::Web => SessionKind::Web,
        }
    }
}

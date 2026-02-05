use crate::bindings;

pub struct SuccessfulLogin {
    pub access_token: String,
    pub refresh_token: Vec<u8>,
}

impl From<SuccessfulLogin> for bindings::SuccessfulLogin {
    fn from(value: SuccessfulLogin) -> Self {
        bindings::SuccessfulLogin {
            access_token: value.access_token,
            refresh_token: value.refresh_token,
        }
    }
}

impl From<bindings::SuccessfulLogin> for SuccessfulLogin {
    fn from(value: bindings::SuccessfulLogin) -> Self {
        SuccessfulLogin {
            access_token: value.access_token,
            refresh_token: value.refresh_token,
        }
    }
}

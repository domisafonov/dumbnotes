use protobuf_common::{MappingError, OptionExt, ProtobufRequestError};

use crate::bindings::{self, successful_login::ExtraToken};

pub enum SuccessfulLogin {
    Api {
        access_token: String,
        refresh_token: Vec<u8>,
    },
    Web {
        access_token: String,
        xsrf_token: Vec<u8>,
    },
}

impl TryFrom<bindings::SuccessfulLogin> for SuccessfulLogin {
    type Error = ProtobufRequestError;
    fn try_from(value: bindings::SuccessfulLogin) -> Result<Self, Self::Error> {
        Ok(
            match value.extra_token.ok_or_mapping_error(MappingError::missing("extra_token"))? {
                ExtraToken::RefreshToken(refresh_token)
                => SuccessfulLogin::Api {
                    access_token: value.access_token,
                    refresh_token,
                },
                ExtraToken::XsrfToken(xsrf_token)
                => SuccessfulLogin::Web {
                    access_token: value.access_token,
                    xsrf_token,
                },
            }
        )
    }
}

impl From<SuccessfulLogin> for bindings::SuccessfulLogin {
    fn from(value: SuccessfulLogin) -> Self {
        match value {
            SuccessfulLogin::Api { access_token, refresh_token }
            => bindings::SuccessfulLogin {
                access_token,
                extra_token: Some(ExtraToken::RefreshToken(refresh_token)),
            },

            SuccessfulLogin::Web { access_token, xsrf_token }
            => bindings::SuccessfulLogin {
                access_token,
                extra_token: Some(ExtraToken::XsrfToken(xsrf_token)),
            }
        }
    }
}

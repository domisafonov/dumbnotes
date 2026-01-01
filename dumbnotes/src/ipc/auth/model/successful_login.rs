use super::super::protobuf;

pub struct SuccessfulLogin {
    pub access_token: String,
    pub refresh_token: Vec<u8>,
}

impl From<SuccessfulLogin> for protobuf::SuccessfulLogin {
    fn from(value: SuccessfulLogin) -> Self {
        protobuf::SuccessfulLogin {
            access_token: value.access_token,
            refresh_token: value.refresh_token,
        }
    }
}

impl From<protobuf::SuccessfulLogin> for SuccessfulLogin {
    fn from(value: protobuf::SuccessfulLogin) -> Self {
        SuccessfulLogin {
            access_token: value.access_token,
            refresh_token: value.refresh_token,
        }
    }
}

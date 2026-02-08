use std::{io::Read, sync::LazyLock};
use reqwest::{IntoUrl, Method, header::{CONTENT_TYPE, HeaderValue}};
use tap::Pipe;

pub static RQ: LazyLock<reqwest::blocking::Client> = LazyLock::new(||
    reqwest::blocking::Client::new()
);

pub trait ReqwestClientExt {
    fn request_pb_successfully<I, O>(
        &self,
        method: Method,
        url: impl IntoUrl,
        auth_token: Option<String>,
        body: impl Into<I>,
    ) -> Result<O, PbReqwestError>
    where
        O: prost::Message + Default,
        I: prost::Message;

    fn post_pb_successfully<I, O>(
        &self,
        url: impl IntoUrl,
        auth_token: Option<String>,
        body: impl Into<I>,
    ) -> Result<O, PbReqwestError>
    where
        O: prost::Message + Default,
        I: prost::Message,
    {
        self.request_pb_successfully(Method::POST, url, auth_token, body)
    }

    fn get_pb_successfully<O: prost::Message + Default>(
        &self,
        url: impl IntoUrl,
        auth_token: Option<String>,
    ) -> Result<O, PbReqwestError>;
}

impl ReqwestClientExt for reqwest::blocking::Client {
    fn request_pb_successfully<I, O>(
        &self,
        method: Method,
        url: impl IntoUrl,
        auth_token: Option<String>,
        body: impl Into<I>,
    ) -> Result<O, PbReqwestError>
    where
        O: prost::Message + Default,
        I: prost::Message,
    {
        self.request(method, url)
            .pb_body(body)
            .pipe(|builder| {
                match auth_token {
                    Some(token) => builder.bearer_auth(token),
                    None => builder,
                }
            })
            .send()?
            .error_for_status()?
            .read_pb::<O>()
    }

    fn get_pb_successfully<O: prost::Message + Default>(
        &self,
        url: impl IntoUrl,
        auth_token: Option<String>,
    ) -> Result<O, PbReqwestError> {
        self.get(url)
            .pipe(|builder| {
                match auth_token {
                    Some(token) => builder.bearer_auth(token),
                    None => builder,
                }
            })
            .send()?
            .error_for_status()?
            .read_pb::<O>()
    }
}

pub trait ReqwestBuilderProtoExt {
    fn pb_body<T: prost::Message>(self, message: impl Into<T>) -> Self;
}
impl ReqwestBuilderProtoExt for reqwest::blocking::RequestBuilder {
    fn pb_body<T: prost::Message>(self, message: impl Into<T>) -> Self {
        let message: T = message.into();
        self
            .header(
                CONTENT_TYPE,
                HeaderValue::from_static("application/protobuf")
            )
            .body(message.encode_to_vec())
    }
}

pub trait ReqwestResponseProtoExt {
    fn read_pb<T: prost::Message + Default>(&mut self) -> Result<T, PbReqwestError>;
}
impl ReqwestResponseProtoExt for reqwest::blocking::Response {
    fn read_pb<T: prost::Message + Default>(&mut self) -> Result<T, PbReqwestError> {
        let mut buf = Vec::with_capacity(16 * 1024);
        self.read_to_end(&mut buf)?;
        T::decode(buf.as_slice())
            .map_err(PbReqwestError::from)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PbReqwestError{
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error("error decoding protobuf response: {0}")]
    Decode(#[from] prost::DecodeError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

mod login;
mod users_notes;
mod note;
mod note_metadata;

#[macro_export]
macro_rules! protobuf_request {
    ($request_type:ty, $model_type:ty) => {
        #[::async_trait::async_trait]
        impl<'r> ::rocket::data::FromData<'r> for $request_type {
            type Error = ::protobuf_common::ProtobufRequestError;

            async fn from_data(
                req: &'r ::rocket::Request<'_>,
                data: ::rocket::Data<'r>,
            ) -> ::rocket::data::Outcome<'r, Self> {
                use ::rocket::data::{Outcome, ToByteUnit};
                use ::rocket::http::Status;
                use ::prost::Message;

                let content_type = ::rocket::http::ContentType::new("application", "protobuf");
                if req.content_type() != Some(&content_type) {
                    return Outcome::Forward((data, Status::UnsupportedMediaType))
                }
                let limit = req.limits()
                    .get("protobuf")
                    .unwrap_or($crate::constants::DEFAULT_PROTOBUF_READ_LIMIT.bytes());
                let result = data.open(limit).into_bytes().await;
                match result {
                    Ok(bytes) if bytes.is_complete() => match <$request_type>::decode(bytes.as_ref()) {
                        Ok(request) => Outcome::Success(request),
                        Err(e) => Outcome::Error((Status::BadRequest, e.into()))
                    }
                    Ok(_) => Outcome::Error((
                        Status::PayloadTooLarge,
                        ::protobuf_common::ProtobufRequestError::RequestTooLarge
                    )),
                    Err(e) => Outcome::Error((Status::BadRequest, e.into())),
                }
            }
        }

        #[::async_trait::async_trait]
        impl<'r> ::rocket::data::FromData<'r> for $model_type {
            type Error = ::protobuf_common::ProtobufRequestError;

            async fn from_data(
                req: &'r ::rocket::Request<'_>,
                data: ::rocket::Data<'r>,
            ) -> ::rocket::data::Outcome<'r, Self> {
                use ::rocket::data::Outcome;
                use ::rocket::http::Status;

                match ::rocket::outcome::try_outcome!(<$request_type>::from_data(req, data).await).try_into() {
                    Ok(mapped) => Outcome::Success(mapped),
                    Err(e) => Outcome::Error((Status::BadRequest, e.into()))
                }
            }
        }
    };
}

#[macro_export]
macro_rules! protobuf_response {
    ($response_type:ty, $model_type:ty) => {
        #[::async_trait::async_trait]
        impl<'r> ::rocket::response::Responder<'r, 'static> for $response_type {
            fn respond_to(
                self,
                _request: &'r ::rocket::Request<'_>,
            ) -> ::rocket::response::Result<'static> {
                use ::prost::Message;

                let serialized = self.encode_to_vec();
                ::rocket::response::Response::build()
                    .header(::rocket::http::ContentType::new("application", "protobuf"))
                    .sized_body(serialized.len(), ::std::io::Cursor::new(serialized))
                    .ok()
            }
        }


        #[::async_trait::async_trait]
        impl<'r> ::rocket::response::Responder<'r, 'static> for $model_type {
            fn respond_to(
                self,
                request: &'r ::rocket::Request<'_>,
            ) -> ::rocket::response::Result<'static> {
                <$response_type>::respond_to(self.into(), request)
            }
        }
    };
}

pub mod bindings {
    include!(concat!(env!("OUT_DIR"), "/dumbnotes.protobuf.rs"));
}

macro_rules! protobuf_request {
    ($request_type:ty) => {
        #[async_trait::async_trait]
        impl<'r> rocket::data::FromData<'r> for $request_type {
            type Error = crate::routes::api::errors::ProtobufRequestError;

            async fn from_data(
                req: &'r rocket::Request<'_>,
                data: rocket::Data<'r>,
            ) -> rocket::data::Outcome<'r, Self> {
                use rocket::data::{Outcome, ToByteUnit};
                use rocket::http::Status;
                use prost::Message;

                let content_type = rocket::http::ContentType::new("application", "protobuf");
                if req.content_type() != Some(&content_type) {
                    return Outcome::Forward((data, Status::UnsupportedMediaType))
                }
                let limit = req.limits()
                    .get("protobuf")
                    .unwrap_or(crate::app_constants::DEFAULT_PROTOBUF_READ_LIMIT.bytes());
                let result = data.open(limit).into_bytes().await;
                match result {
                    Ok(bytes) if bytes.is_complete() => match <$request_type>::decode(bytes.as_ref()) {
                        Ok(request) => Outcome::Success(request),
                        Err(e) => Outcome::Error((Status::BadRequest, e.into()))
                    }
                    Ok(_) => Outcome::Error((
                        Status::PayloadTooLarge,
                        crate::routes::api::errors::ProtobufRequestError::TooLarge
                    )),
                    Err(e) => Outcome::Error((Status::BadRequest, e.into())),
                }
            }
        }
    };
}

macro_rules! protobuf_response {
    ($response_type:ty) => {
        #[async_trait::async_trait]
        impl<'r> rocket::response::Responder<'r, 'static> for $response_type {
            fn respond_to(
                self,
                _request: &'r rocket::Request<'_>,
            ) -> rocket::response::Result<'static> {
                use prost::Message;

                let serialized = self.encode_to_vec();
                rocket::response::Response::build()
                    .header(rocket::http::ContentType::new("application", "protobuf"))
                    .sized_body(serialized.len(), std::io::Cursor::new(serialized))
                    .ok()
            }
        }
    };
}

protobuf_request!(bindings::LoginRequest);
protobuf_response!(bindings::LoginResponse);

#[macro_export]
macro_rules! gen_proto_ipc_wrappers {
    (
        $input_container_binding:ty[$input_name:ident] | $input_binding:ty => $input_vis:vis $input_container_wrapper:ident,
        $output_container_binding:path[$output_name:ident] | $output_binding:ty => $output_vis:vis $output_wrapper:ident$(,)?
    ) => {
        #[derive(Debug)]
        $input_vis struct $input_container_wrapper($input_container_binding);

        #[derive(Debug)]
        $output_vis struct $output_wrapper($output_binding);

        impl ::dumbnotes::ipc::data::IpcInputContainerWrapper<$input_binding, $input_container_binding> for $input_container_wrapper {
            fn get_id(&self) -> u64 {
                self.0.command_id
            }

            fn get_input(
                &self,
            ) -> ::std::result::Result<$input_binding, ::protobuf_common::ProtobufRequestError> {
                Self::take_input_from_maybe_container(self.0.clone())
            }

            fn into_id_and_input(
                self,
            ) -> (u64, ::std::result::Result<$input_binding, ::protobuf_common::ProtobufRequestError>) {
                (
                    self.0.command_id,
                    Self::take_input_from_maybe_container(self.0),
                )
            }

            fn wrap(wrapped: $input_container_binding) -> Self {
                $input_container_wrapper(wrapped)
            }
        }

        impl $input_container_wrapper {
            fn take_input_from_maybe_container(
                container: $input_container_binding,
            ) -> ::std::result::Result<$input_binding, ::protobuf_common::ProtobufRequestError> {
                use ::protobuf_common::{MappingError, OptionExt};
                container.$input_name.ok_or_mapping_error(MappingError::missing(stringify!($input_name)))
            }
        }

        impl dumbnotes::ipc::data::IpcOutput<$output_binding, $output_container_binding> for $output_wrapper {
            fn into_container(self, command_id: u64) -> $output_container_binding {
                $output_container_binding {
                    command_id,
                    $output_name: Some(self.0),
                }
            }
        }
    };
}

#[macro_export]
macro_rules! check_access_token {
    ($request_name:expr, $access_token_validator:ident, $access_token:ident, $invalid_credentials_error:expr$(,)*) => {
        match $access_token_validator.check_access_token(&$access_token) {
            ::std::result::Result::Ok(data) => data,
            Err(::dumbnotes::access_token::AccessTokenValidatorError::InvalidToken(_)) => {
                ::log::warn!("attempted \"{}\" request using invalid access token: {}", $request_name, $access_token);
                return ::std::result::Result::Ok($invalid_credentials_error)
            },
            Err(::dumbnotes::access_token::AccessTokenValidatorError::ExpiredToken(_)) => {
                ::log::warn!("attempted \"{}\" request using expired token", $request_name);
                return ::std::result::Result::Ok($invalid_credentials_error)
            },
        }
    };
}

#[macro_export]
macro_rules! assert_json_eq {
    ($value:expr, $json_string:expr$(,)?) => {
        match ($value, $json_string) {
            (ref value, ref json_string) => {
                use $crate::test::Infer;
                let json_left = serde_json::to_value($value)
                    .unwrap_or_else(|e|
                        panic!("failed to deserialize \"{:?}\": {e}", $value)
                    );
                let deserialized_right = value.same_inferred_type(
                    serde_json
                        ::from_str(json_string.as_ref())
                        .unwrap_or_else(|e|
                            panic!("failed to deserialize \"{:?}\": {e}", json_string)
                        ),
                );
                let json_right = serde_json::to_value(deserialized_right)
                    .unwrap_or_else(|e|
                        panic!("failed to reserialize \"{:?}\": {e}", $json_string)
                    );
                if json_left != json_right {
                    panic!(
                        r#"assertion `left == right` failed
       left: {:?}
  json_left: {:?}
      right: {:?}
 json_right: {:?}"#,
                        $value,
                        json_left,
                        $json_string,
                        json_right,
                    )
                }
            }
        }
    };
}

#[doc(hidden)]
pub trait Infer: Sized {
    type T;
    fn same_inferred_type(&self, value: Self) -> Self {
        value
    }
}
impl<T> Infer for T {
    type T = T;
}

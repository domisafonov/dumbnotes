use rand::{Rng, RngExt};
use uuid::{Uuid, Variant, Version};

// https://github.com/rust-lang/rust/issues/130113
pub fn send_fut_lifetime_workaround<F: Future + Send>(
    fut: F,
) -> impl Future<Output=F::Output> + Send {
    fut
}

#[macro_export]
macro_rules! error_exit {
    ($($args:tt)*) => ({
        log::error!($($args)*);
        std::process::exit(1)
    });
}

pub fn make_uuid<R: Rng>(rng: &mut R) -> Uuid {
    uuid::Builder::from_random_bytes(rng.random())
        .with_variant(Variant::RFC4122)
        .with_version(Version::Random)
        .into_uuid()
}

pub trait OptionStrRefExt {
    fn as_str_ref(&self) -> Option<&str>;
}
impl<S: AsRef<str>> OptionStrRefExt for Option<S> {
    fn as_str_ref(&self) -> Option<&str> {
        match self {
            Some(v) => Some(v.as_ref()),
            None => None,
        }
    }
}

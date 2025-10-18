#[cfg(test)] pub mod testing;

use rand::Rng;
use uuid::{Uuid, Variant, Version};

pub fn make_uuid<R: Rng>(rng: &mut R) -> Uuid {
    uuid::Builder::from_random_bytes(rng.random())
        .with_variant(Variant::RFC4122)
        .with_version(Version::Random)
        .into_uuid()
}

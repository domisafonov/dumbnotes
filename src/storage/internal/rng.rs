use std::sync::{Arc, Mutex};
use rand::{Rng, RngCore};
use uuid::{Uuid, Variant, Version};

pub fn make_uuid(rng: &mut impl Rng) -> Uuid {
    uuid::Builder::from_u128(rng.random::<u128>())
        .with_variant(Variant::RFC4122)
        .with_version(Version::Random)
        .into_uuid()
}

pub struct SyncRng {
    pub rng: Arc<Mutex<dyn RngCore>>,
}

impl SyncRng {
    pub fn new(rng: impl RngCore + 'static) -> Self {
        SyncRng {
            rng: Arc::new(Mutex::new(rng)),
        }
    }
}

unsafe impl Send for SyncRng {}

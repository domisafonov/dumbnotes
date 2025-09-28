use std::ops::Deref;
use std::sync::{Arc, Mutex, MutexGuard};
use rand::Rng;
use uuid::{Uuid, Variant, Version};

pub fn make_uuid(rng: &mut impl Rng) -> Uuid {
    uuid::Builder::from_u128(rng.random::<u128>())
        .with_variant(Variant::RFC4122)
        .with_version(Version::Random)
        .into_uuid()
}

pub struct SyncRng<R: Rng> {
    rng: Arc<Mutex<R>>,
}

impl<R: Rng> SyncRng<R> {
    pub fn new(rng: R) -> Self {
        SyncRng {
            rng: Arc::new(Mutex::new(rng)),
        }
    }

    pub fn get_rng(&self) -> MutexGuard<'_, R> {
        self.rng.lock().unwrap()
    }
}

impl<R: Rng> Deref for SyncRng<R> {
    type Target = Arc<Mutex<R>>;

    fn deref(&self) -> &Self::Target {
        &self.rng
    }
}

impl<R: Rng> Clone for SyncRng<R> {
    fn clone(&self) -> Self {
        SyncRng {
            rng: self.rng.clone(),
        }
    }
}

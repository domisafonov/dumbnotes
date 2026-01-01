use std::ops::Deref;
use std::sync::{Arc, Mutex, MutexGuard};
use argon2::password_hash::rand_core::TryCryptoRng;

pub struct SyncRng<R: TryCryptoRng + Send + Sync> {
    rng: Arc<Mutex<R>>,
}

impl<R: TryCryptoRng + Send + Sync> SyncRng<R> {
    pub fn new(rng: R) -> Self {
        SyncRng {
            rng: Arc::new(Mutex::new(rng)),
        }
    }

    pub fn get_rng(&self) -> MutexGuard<'_, R> {
        self.rng.lock().unwrap()
    }
}

impl<R: TryCryptoRng + Send + Sync> Deref for SyncRng<R> {
    type Target = Arc<Mutex<R>>;

    fn deref(&self) -> &Self::Target {
        &self.rng
    }
}

impl<R: TryCryptoRng + Send + Sync> Clone for SyncRng<R> {
    fn clone(&self) -> Self {
        SyncRng {
            rng: self.rng.clone(),
        }
    }
}
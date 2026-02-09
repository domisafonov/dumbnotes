use std::sync::atomic::{AtomicU16, Ordering};

static NEXT_PORT: AtomicU16 = AtomicU16::new(8000);

thread_local! {
    pub static LOCAL_PORT: u16 = NEXT_PORT.fetch_add(1, Ordering::Relaxed);
}

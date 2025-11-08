mod errors;
mod internal;
mod events;

use futures::Stream;
use notify::RecommendedWatcher;
use std::path::Path;
pub use errors::*;
pub use events::Event;
use internal::FileWatcherImpl;

pub trait FileWatcher: Send + Sync + Clone + 'static {
    type Guard: FileWatchGuard;

    fn watch(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<Self::Guard, FileWatcherError>;
}

pub trait FileWatchGuard: Send + Sync {
    fn get_events(&self) -> impl Stream<Item=Result<Event, FileWatcherError>> + Send + 'static;

    /// Trigger one modification event, skip one actual event afterward.
    fn trigger_modification(&self);
}

pub type ProductionFileWatcher = FileWatcherImpl<RecommendedWatcher>;
impl ProductionFileWatcher {
    pub fn new() -> Result<Self, FileWatcherError> {
        FileWatcherImpl::new_impl()
    }
}

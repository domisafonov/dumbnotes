use thiserror::Error;

#[derive(Debug, Error)]
pub enum FileWatcherError {
    #[error("failed to create file watcher: {0}")]
    Init(notify::Error),

    #[error("failed to start watching file: {0}")]
    WatchStart(notify::Error),

    #[error("failed to watch file: {0}")]
    Watch(String),

    #[error("too many file watcher events, discarding {0}")]
    Overflow(u64),
}
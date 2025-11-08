use async_stream::try_stream;
use futures::Stream;
use notify::{EventKind, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{new_debouncer_opt, DebounceEventHandler, DebounceEventResult, DebouncedEvent, Debouncer, RecommendedCache};
use std::path::{Path, PathBuf};
use std::pin::pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use futures::future::{select, Either};
use thiserror::Error;
use tokio::sync::{broadcast, Notify};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event {
    // TODO: make separate create/modify and remove events
    Any,
}

#[derive(Clone, Debug)]
enum InternalEvent {
    Event(DebouncedEvent),
    Error(String),
}

pub trait FileWatcher: Send + Sync + Clone {
    type Guard: FileWatchGuard;

    fn watch(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<Self::Guard, FileWatcherError>;
}

pub struct FileWatcherImpl<W: notify::Watcher> {
    inner: Arc<Mutex<FileWatcherInternal<W>>>,
}

impl<W: notify::Watcher> Clone for FileWatcherImpl<W> {
    fn clone(&self) -> Self {
        FileWatcherImpl {
            inner: self.inner.clone(),
        }
    }
}

struct FileWatcherInternal<W: notify::Watcher> {
    watcher: Debouncer<W, RecommendedCache>,
    events: broadcast::Sender<InternalEvent>,
}

impl<W: notify::Watcher + Send + Sync> FileWatcher for FileWatcherImpl<W> {
    type Guard = FileWatchGuardImpl<W>;

    fn watch(
        &self,
        path: impl AsRef<Path>
    ) -> Result<FileWatchGuardImpl<W>, FileWatcherError> {
        let path = path.as_ref().to_owned();
        let mut inner = self.inner
            .lock()
            .expect("failed locking the file watcher");
        inner
            .watcher
            .watch(&path, RecursiveMode::NonRecursive)
            .map_err(FileWatcherError::WatchStart)?;
        Ok(
            FileWatchGuardImpl {
                path,
                file_watcher: self.inner.clone(),
                trigger_modification: Arc::new(Notify::new()),
            }
        )
    }
}

impl<W: notify::Watcher> FileWatcherImpl<W> {
    pub fn new_impl() -> Result<Self, FileWatcherError> {
        let (sender, _) = broadcast::channel(16);
        Ok(
            FileWatcherImpl {
                inner: Arc::new(
                    Mutex::new(
                        FileWatcherInternal {
                            watcher: new_debouncer_opt(
                                Duration::from_secs(10),
                                None,
                                Callback(sender.clone()),
                                Default::default(),
                                Default::default(),
                            )
                                .map_err(FileWatcherError::Init)?,
                            events: sender,
                        }
                    )
                ),
            }
        )
    }
}

pub struct Callback(broadcast::Sender<InternalEvent>);
impl DebounceEventHandler for Callback {
    fn handle_event(&mut self, event: DebounceEventResult) {
        // TODO: the only possible error is not having subscribers
        //  log receiving event after unsub
        match event {
            Ok(v) => v.into_iter().for_each(|v| {
                let _ = self.0.send(InternalEvent::Event(v));
            }),
            Err(e) => e.into_iter().for_each(|e| {
                let _ = self.0.send(InternalEvent::Error(e.to_string()));
            })
        }
    }
}

pub trait FileWatchGuard: Send + Sync {
    fn get_events(&self) -> impl Stream<Item=Result<Event, FileWatcherError>> + Send + 'static;

    /// Trigger one modification event, skip one actual event afterward.
    fn trigger_modification(&self);
}

pub struct FileWatchGuardImpl<W: notify::Watcher> {
    path: PathBuf,
    file_watcher: Arc<Mutex<FileWatcherInternal<W>>>,
    trigger_modification: Arc<Notify>,
}

impl<W: notify::Watcher> Drop for FileWatchGuardImpl<W> {
    fn drop(&mut self) {
        self.file_watcher
            .lock().expect("failed locking the file watcher")
            .watcher
            .unwatch(&self.path).expect("failed to unwatch");
    }
}

impl<W: notify::Watcher + Send + Sync> FileWatchGuard for FileWatchGuardImpl<W> {
    fn get_events(&self) -> impl Stream<Item=Result<Event, FileWatcherError>> + Send + 'static {
        let lock = self.file_watcher
            .lock().expect("failed locking the file watcher");
        let mut receiver = lock
            .events
            .subscribe();
        drop(lock);
        
        let trigger_modification = self.trigger_modification.clone();

        try_stream! {
            let mut do_drop_one = false;

            loop {
                let event = match select(
                    pin!(trigger_modification.notified()),
                    pin!(receiver.recv())
                ).await {
                    Either::Left(_) => {
                        do_drop_one = true;
                        Some(Event::Any)
                    },
                    Either::Right((event, _)) => match event {
                        Ok(InternalEvent::Event(event)) => match event.kind {
                            EventKind::Create(_) |
                                EventKind::Modify(_)
                            => Some(Event::Any),

                            EventKind::Remove(_) => Some(Event::Any),

                            _ => None,
                        },
                        Ok(InternalEvent::Error(message)) => Err(FileWatcherError::Watch(message))?,
                        Err(e) => match e {
                            broadcast::error::RecvError::Lagged(n) => Err(FileWatcherError::Overflow(n))?,
                            _ => unreachable!(),
                        },
                    }
                };
                if let Some(e) = event {
                    if do_drop_one {
                        do_drop_one = false;
                    } else {
                        yield e;
                    }
                }
            }
        }
    }

    fn trigger_modification(&self) {
        self.trigger_modification.notify_one()
    }
}

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

pub type ProductionFileWatcher = FileWatcherImpl<RecommendedWatcher>;
impl ProductionFileWatcher {
    pub fn new() -> Result<Self, FileWatcherError> {
        FileWatcherImpl::new_impl()
    }
}

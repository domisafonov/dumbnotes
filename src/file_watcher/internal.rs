#[cfg(test)] mod tests;

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use notify_debouncer_full::{new_debouncer_opt, DebounceEventHandler, DebounceEventResult, DebouncedEvent, Debouncer, RecommendedCache};
use tokio::sync::{broadcast, Notify};
use notify::{EventKind, RecursiveMode};
use futures::Stream;
use async_stream::try_stream;
use log::{debug, error, log_enabled, trace};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use crate::file_watcher::{Event, FileWatchGuard, FileWatcher, FileWatcherError};
use crate::lib_constants::FILE_WATCHER_DEBOUNCE_TIME;

const FILE_WATCHER_BUFFER_SIZE: usize = 16;

#[derive(Clone, Debug)]
enum InternalEvent {
    Event(DebouncedEvent),
    Error {
        message: String,
        paths: Vec<PathBuf>,
    },
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

impl<W: notify::Watcher + Send + Sync + 'static> FileWatcher for FileWatcherImpl<W> {
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
        debug!("starting to watch path \"{}\"", path.display());
        Ok(
            FileWatchGuardImpl {
                path,
                file_watcher: self.inner.clone(),
                skip_modification: Arc::new(Notify::new()),
            }
        )
    }
}

impl<W: notify::Watcher> FileWatcherImpl<W> {
    pub fn new_impl() -> Result<Self, FileWatcherError> {
        let (sender, _) = broadcast::channel(FILE_WATCHER_BUFFER_SIZE);
        Ok(
            FileWatcherImpl {
                inner: Arc::new(
                    Mutex::new(
                        FileWatcherInternal {
                            watcher: new_debouncer_opt(
                                FILE_WATCHER_DEBOUNCE_TIME.unsigned_abs(),
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

struct Callback(broadcast::Sender<InternalEvent>);
impl DebounceEventHandler for Callback {
    fn handle_event(&mut self, event: DebounceEventResult) {
        match event {
            Ok(v) => v.into_iter().for_each(|v| {
                trace!("file event received: {v:?}");
                // SAFETY: the only possible error is not having subscribers
                let _ = self.0.send(InternalEvent::Event(v));
            }),

            Err(e) => e.into_iter().for_each(|e| {
                error!("file watching error: {e}");
                // SAFETY: the only possible error is not having subscribers
                let _ = self.0.send(
                    InternalEvent::Error {
                        message: e.to_string(),
                        paths: e.paths,
                    }
                );
            })
        }
    }
}

pub struct FileWatchGuardImpl<W: notify::Watcher> {
    path: PathBuf,
    file_watcher: Arc<Mutex<FileWatcherInternal<W>>>,
    skip_modification: Arc<Notify>,
}

impl<W: notify::Watcher> Drop for FileWatchGuardImpl<W> {
    fn drop(&mut self) {
        debug!("stopping watching path \"{}\"", self.path.display());
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
        let path = self.path.clone();
        let mut receiver = BroadcastStream
            ::from(lock.events.subscribe())
            .filter(move |event| match event {
                Ok(InternalEvent::Event(event)) => event.paths.contains(&path),
                Ok(InternalEvent::Error { paths, .. }) => paths.contains(&path),
                _ => true
            });
        drop(lock);

        let path_display = if log_enabled!(log::Level::Trace) {
            Cow::Owned(format!("{}", self.path.display()))
        } else {
            Cow::Borrowed("")
        };
        let skip_modification = self.skip_modification.clone();
        try_stream! {
            let mut do_drop_one = false;

            trace!("observing file changes for path \"{path_display}\"");
            loop {
                let event = tokio::select! {
                    biased;
                    _ = skip_modification.notified() => {
                        do_drop_one = true;
                        Ok(None)
                    },
                    event = receiver.next() => {
                        match event.unwrap() {
                            Ok(InternalEvent::Event(event)) => match event.kind {
                                EventKind::Create(_) |
                                    EventKind::Modify(_)
                                => Ok(Some(Event::Any)),

                                EventKind::Remove(_) => Ok(Some(Event::Any)),

                                _ => Ok(None),
                            },
                            Ok(InternalEvent::Error { message, .. }) => Err(FileWatcherError::Watch(message)),
                            Err(e) => match e {
                                tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n)
                                => Err(FileWatcherError::Overflow(n)),
                            },
                        }
                    },
                }?;
                if let Some(e) = event {
                    if do_drop_one {
                        do_drop_one = false;
                        trace!("dropping event {e:?}");
                    } else {
                        trace!("event on path \"{}\": {e:?}", path_display);
                        yield e
                    }
                }
            }
        }
    }

    fn skip_modification(&self) {
        trace!("skip_modification() called for {}", self.path.display());
        self.skip_modification.notify_one()
    }
}

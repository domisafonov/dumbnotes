use std::{io, thread};
use std::io::Read;
use std::mem::replace;
use std::os::fd::AsRawFd;
use std::string::FromUtf8Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use boolean_enums::gen_boolean_enum;
use thiserror::Error;
use unix::FdNonblockExt;
use crate::constants::BACKGROUND_READER_CHECK_INTERVAL;

pub trait Reader: AsRawFd + Read + Send + Sync + 'static {}
impl<T: AsRawFd + Read + Send + Sync + 'static> Reader for T {}

pub struct BackgroundReader<R: Reader> {
    thread: Option<JoinHandle<Result<(), BackgroundReaderError>>>,
    inner: Arc<Inner<R>>,
}

struct Inner<R: Reader> {
    mutable: Mutex<InnerMut<R>>,
    shutdown_notice: AtomicBool,
    timeout: Option<Duration>,
}

struct InnerMut<R: Reader> {
    reader: R,
    buf: Vec<u8>,
}

impl<R: Reader> BackgroundReader<R> {
    pub fn new(
        reader: R,
        timeout: Option<u64>,
    ) -> Result<Self, BackgroundReaderError> {
        let inner = Arc::new(
            Inner {
                mutable: Mutex::new(
                    InnerMut {
                        reader,
                        buf: Vec::with_capacity(16 * 1024),
                    }
                ),
                shutdown_notice: AtomicBool::new(false),
                timeout: timeout.map(Duration::from_millis),
            }
        );
        let inner2 = inner.clone();
        let thread = thread::spawn(move || Self::read_loop(inner2));
        Ok(
            Self {
                thread: Some(thread),
                inner,
            }
        )
    }

    fn read_loop(inner: Arc<Inner<R>>) -> Result<(), BackgroundReaderError> {
        let mut read_buf = [0u8; 16 * 1024];

        inner.mutable.lock()
            .expect("couldn't lock the reader's state")
            .reader
            .set_nonblock(true)
            .map_err(BackgroundReaderError::Io)?;

        loop {
            if inner.shutdown_notice.load(Ordering::Relaxed) {
                return Ok(());
            }
            let mut mutables = inner.mutable.lock()
                .expect("couldn't lock the reader's state");
            match mutables.reader.read(&mut read_buf) {
                Ok(0) => return Ok(()),

                Ok(bytes_read)
                => mutables.buf.extend_from_slice(&read_buf[..bytes_read]),

                Err(e) if e.kind() != io::ErrorKind::WouldBlock
                    && e.kind() != io::ErrorKind::Interrupted
                => return Err(BackgroundReaderError::Io(e)),

                Err(_) => {
                    drop(mutables);
                    thread::sleep(BACKGROUND_READER_CHECK_INTERVAL)
                },
            }
        }
    }

    pub fn take(&mut self) -> Vec<u8> {
        let mut mutables = self.inner.mutable.lock()
            .expect("couldn't lock the reader's state");
        replace(&mut mutables.buf, Vec::with_capacity(16 * 1024))
    }

    pub fn read_to_end_bytes(mut self) -> Result<Vec<u8>, BackgroundReaderError> {
        let Some(thread) = self.thread.take() else {
            return Ok(Vec::new())
        };
        thread.join().expect("background reader thread panicked")?;
        Ok(
            std::mem::take(
                &mut self.inner.mutable.lock()
                    .unwrap_or_else(|e|
                        panic!("background reader thread panicked: {e}")
                    )
                    .buf
            )
        )
    }

    pub fn read_to_end(self) -> Result<String, BackgroundReaderError> {
        String::from_utf8(self.read_to_end_bytes()?)
            .map_err(BackgroundReaderError::from)
    }

    pub fn wait_until_bytes(
        &mut self,
        bytes: &[u8],
    ) -> Result<Vec<u8>, BackgroundReaderError> {
        self
            .wait_until_bytes_impl(
                bytes,
                IsString::No,
            )
    }

    fn wait_until_bytes_impl(
        &mut self,
        bytes: &[u8],
        is_string: IsString,
    ) -> Result<Vec<u8>, BackgroundReaderError> {
        let mut current_pos = 0usize;
        let time = Instant::now();

        loop {
            let mut mutables = self.inner.mutable.lock()
                .unwrap_or_else(|e|
                    panic!("background reader thread panicked: {e}")
                );
            let buf = &mutables.buf[current_pos..];
            let mut found = None;
            for w in buf.windows(bytes.len()) {
                if w == bytes {
                    found = Some(current_pos);
                    break;
                } else {
                    current_pos += 1;
                }
            }
            if let Some(pos) = found {
                let mut new_buf = Vec::<u8>::with_capacity(16 * 1024);
                new_buf.extend_from_slice(&mutables.buf[pos + bytes.len()..]);
                std::mem::swap(&mut new_buf, &mut mutables.buf);
                new_buf.truncate(pos + bytes.len());
                return Ok(new_buf);
            }
            drop(mutables);

            if let Some(ref timeout) = self.inner.timeout
                && time.elapsed() > *timeout
            {
                let mutables = self.inner.mutable.lock()
                    .unwrap_or_else(|e|
                        panic!("background reader thread panicked: {e}")
                    );
                let stringified = if is_string.into() {
                    String::from_utf8_lossy(&mutables.buf).into_owned()
                } else {
                    let mut string = String::with_capacity(mutables.buf.len() * 2);
                    for byte in &mutables.buf {
                        let formatted = format!("{byte:02x} ");
                        string.push_str(&formatted);
                    }
                    string
                };
                drop(mutables);
                eprintln!("last log messages: {stringified}");
                panic!("timeout expired")
            }
            thread::sleep(BACKGROUND_READER_CHECK_INTERVAL);
        }
    }

    pub fn wait_until(
        &mut self,
        string: &str,
    ) -> Result<String, BackgroundReaderError> {
        String
            ::from_utf8(
                self
                    .wait_until_bytes_impl(
                        string.as_bytes(),
                        IsString::Yes,
                    )?
            )
            .map_err(BackgroundReaderError::from)
    }
}
gen_boolean_enum!(IsString);

impl<R: Reader> Drop for BackgroundReader<R> {
    fn drop(&mut self) {
        let Some(thread) = self.thread.take() else {
            return
        };
        self.inner.shutdown_notice.store(true, Ordering::Relaxed);
        thread.join()
            .expect("background reader thread panicked")
            .unwrap_or_else(|e|
                panic!("background reader thread failed: {e}")
            )
    }
}

#[derive(Debug, Error)]
pub enum BackgroundReaderError {
    #[error(transparent)]
    Io(io::Error),

    #[error(transparent)]
    FromUtf8(#[from] FromUtf8Error),
}

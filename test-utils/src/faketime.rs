use std::{fs::{File, FileTimes, OpenOptions}, io, path::{Path, PathBuf}, process::Command, sync::atomic::{AtomicU64, Ordering}, time::SystemTime};

const CLEANUP_IN_PROGRESS: u64 = u64::MAX;

static FAKETIME_CLEANUP: FaketimeCleanup = FaketimeCleanup {
    ref_count: AtomicU64::new(0),
};

struct FaketimeCleanup {
    ref_count: AtomicU64,
}

impl FaketimeCleanup {
    fn inc_count(&self) {
        while self.try_inc_count().is_err() {};
    }

    fn try_inc_count(&self) ->Result<u64, u64> {
        self.ref_count
            .try_update(
                Ordering::Relaxed,
                Ordering::Relaxed,
                |current_count| {
                    if current_count == CLEANUP_IN_PROGRESS {
                        None
                    } else {
                        Some(current_count + 1)
                    }
                },
            )
    }

    fn dec_count(&self) {
        let old_count = self.ref_count
            .update(
                Ordering::Relaxed,
                Ordering::Relaxed,
                |current_count| {
                    assert_ne!(
                        current_count,
                        0,
                        "ref count decreased below 0",
                    );
                    assert_ne!(
                        current_count,
                        CLEANUP_IN_PROGRESS,
                        "ref count decreased below 0 while cleaning up",
                    );

                    if current_count == 1 {
                        CLEANUP_IN_PROGRESS
                    } else {
                        current_count - 1
                    }
                }
            );
        if old_count != 1 {
            return
        }

        let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("scripts/run_with_faketime.fish");
        Command::new(script)
            .arg("--cleanup")
            .status()
            .expect("error cleaning up faketime shm junk");

        self.ref_count.store(0, Ordering::Relaxed);
    }
}

pub struct Faketime {
    timestamp_file: File,
}

impl Faketime {
    pub fn new(
        timestamp_file: impl AsRef<Path>,
    ) -> Result<Faketime, io::Error> {
        FAKETIME_CLEANUP.inc_count();
        Ok(
            Faketime {
                timestamp_file: OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(timestamp_file)?,
            }
        )
    }

    pub fn set_time(&self, time: SystemTime) -> Result<(), io::Error> {
        self
            .timestamp_file
            .set_times(
                FileTimes::new()
                    .set_modified(time)
            )?;
        Ok(())
    }
}

impl Drop for Faketime {
    fn drop(&mut self) {
        FAKETIME_CLEANUP.dec_count();
    }
}

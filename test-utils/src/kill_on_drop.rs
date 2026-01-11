use std::ops::{Deref, DerefMut};
use std::process::Child;
use std::thread;
use std::time::Instant;
use unix::ChildKillTermExt;
use crate::constants::{KILL_CHECK_INTERVAL, TERM_WAIT};

#[derive(Debug)]
pub struct KillOnDropChild(Option<Child>);

impl KillOnDropChild {
    pub fn into_child(mut self) -> Child {
        self.0.take()
            .expect("invalid KillOnDropChild with None wrapped")
    }
}

impl Drop for KillOnDropChild {
    fn drop(&mut self) {
        let Some(ref mut child) = self.0 else {
            return
        };

        match child.try_wait() {
            Ok(Some(_)) => return,
            Err(e) => {
                eprintln!("failed checking child process's status: {e}");
                return
            },
            _ => ()
        }

        let id = child.id();

        if let Err(e) = child.kill_term() {
            eprintln!(
                "leaking child process {id} due to failure \
                    to send SIGTERM {id}: {e}",
            );
            return
        }

        let wait_start = Instant::now();
        while wait_start.elapsed() < TERM_WAIT {
            match child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => thread::sleep(KILL_CHECK_INTERVAL),
                Err(e) => {
                    eprintln!("failed waiting for child process: {e}");
                    return
                }
            }
        }

        eprintln!(
            "child process {id} refused to stop in {} milliseconds, killing",
            TERM_WAIT.as_millis(),
        );


        if let Err(e) = child.kill() {
            eprintln!(
                "leaking child process {id} due to failure \
                    to send SIGKILL {id}: {e}",
            );
        }
    }
}

impl Deref for KillOnDropChild {
    type Target = Child;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
            .expect("invalid KillOnDropChild with None wrapped")
    }
}

impl DerefMut for KillOnDropChild {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
            .expect("invalid KillOnDropChild with None wrapped")
    }
}

pub trait ChildKillOnDropExt {
    fn kill_on_drop(self) -> KillOnDropChild;
}
impl ChildKillOnDropExt for Child {
    fn kill_on_drop(self) -> KillOnDropChild {
        KillOnDropChild(Some(self))
    }
}

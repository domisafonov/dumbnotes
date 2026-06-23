use std::{io::Error, process::ExitStatus};

use boolean_enums::gen_boolean_enum;
use tokio::{process::Child, time::Instant};
use unix::kill_term;

pub trait KillWithTimeoutChildExt {
    fn kill_with_timeout(
        &mut self,
        deadline: Instant,
        send_term_first: SendTerm,
    ) -> impl Future<Output=Result<ExitStatus, Error>>
        + Send
        + Sync;
}
impl KillWithTimeoutChildExt for Child {
    async fn kill_with_timeout(
        &mut self,
        deadline: Instant,
        send_term_first: SendTerm,
    ) -> Result<ExitStatus, Error> {
        if send_term_first.into() {
            let pid = if let Some(pid) = self.id() {
                pid
            } else {
                return Ok(
                    self.try_wait()?
                        .expect("process state invalid")
                )
            };
            kill_term(pid)?;
        }

        let termination = tokio::time::timeout_at(
            deadline,
            self.wait(),
        );
        if let Ok(exit_status) = termination.await {
            return Ok(exit_status?)
        }

        self.kill().await?;
        Ok(
            self.try_wait()?
                .expect("process state invalid")
        )
    }
}
gen_boolean_enum!(pub SendTerm);

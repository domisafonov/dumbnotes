use rexpect::session::PtySession;
use rexpect::process::wait::WaitStatus;
use std::error::Error;

pub trait PtySessionExt {
    fn assert_exit_success(&mut self) -> Result<String, Box<dyn Error>> {
        let remaining_output = self.get_exit_code()?;
        assert_eq!(remaining_output.exit_code, 0);
        Ok(remaining_output.remaining_output)
    }

    fn assert_exit_failure(&mut self) -> Result<String, Box<dyn Error>> {
        let remaining_output = self.get_exit_code()?;
        assert_ne!(remaining_output.exit_code, 0);
        Ok(remaining_output.remaining_output)
    }

    fn get_exit_code(&mut self) -> Result<ExitCodeResult, Box<dyn Error>>;
}

#[derive(Debug)]
pub struct ExitCodeResult {
    remaining_output: String,
    exit_code: i32,
}

impl PtySessionExt for PtySession {
    fn get_exit_code(&mut self) -> Result<ExitCodeResult, Box<dyn Error>> {
        let remaining_output = self.exp_eof()?;
        let result = self.process.wait()?;
        match result {
            WaitStatus::Exited(_, exit_code) => 
                Ok(
                    ExitCodeResult {
                        remaining_output,
                        exit_code,
                    }
                ),
            _ => panic!("failed to get exit code"),
        }
    }
}

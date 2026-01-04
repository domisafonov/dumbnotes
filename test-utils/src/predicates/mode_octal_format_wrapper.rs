use std::fmt::{Display, Formatter};
use std::mem::transmute;

pub enum EvalResult {
    Success(u32),
    IoError(std::io::Error),
    Failure(u32),
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub(super) struct ModeOctalFormatWrapper(u32);

pub(super) trait ModeExt {
    fn octal_format_wrapper(&self) -> &ModeOctalFormatWrapper;
}

impl ModeExt for u32 {
    fn octal_format_wrapper(&self) -> &ModeOctalFormatWrapper {
        unsafe { transmute::<&u32, &ModeOctalFormatWrapper>(self)}
    }
}

impl Display for ModeOctalFormatWrapper {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#03o}", self.0)
    }
}

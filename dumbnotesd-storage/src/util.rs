pub trait StrExt: AsRef<str> {
    fn nonblank_to_some(&self) -> Option<String> {
        Some(self.as_ref().trim())
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    }
}

impl<T: AsRef<str>> StrExt for T {}

use language_tags::LanguageTag;

#[derive(Debug)]
pub struct Translator(pub &'static LanguageTag);

impl From<&'static LanguageTag> for Translator {
    fn from(value: &'static LanguageTag) -> Self {
        Translator(value)
    }
}

impl Translator {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

macro_rules! t {
    ($translator:expr, $($all:tt)*) => {
        rust_i18n::t!($($all)*, locale = $translator.0.as_str())
    }
}
pub(crate) use t;

use std::fmt::{Display, Formatter};
use std::mem::transmute;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use predicates::prelude::*;
use predicates::reflection::{Case, Parameter, PredicateReflection, Product};

#[derive(Clone, Debug)]
pub struct FileModePredicate {
    required: u32,
    prohibited: u32,
}

pub fn file_mode(
    required: u32,
    prohibited: u32,
) -> FileModePredicate {
    FileModePredicate {
        required,
        prohibited,
    }
}

impl FileModePredicate {
    fn eval_impl(&self, variable: &Path) -> EvalResult {
        let metadata = match variable.metadata() {
            Ok(metadata) => metadata,
            Err(e) => return EvalResult::IoError(e),
        };
        let mode = metadata.mode();
        if mode & self.required == self.required
            && mode & self.prohibited == 0
        {
            EvalResult::Success(mode)
        } else {
            EvalResult::Failure(mode)
        }
    }
}

enum EvalResult {
    Success(u32),
    IoError(std::io::Error),
    Failure(u32),
}

impl Predicate<Path> for FileModePredicate {
    fn eval(&self, variable: &Path) -> bool {
        matches!(self.eval_impl(variable), EvalResult::Success(_))
    }

    fn find_case<'a>(&'a self, expected: bool, variable: &Path) -> Option<Case<'a>> {
        let actual = self.eval_impl(variable);
        let actual_bool = matches!(actual, EvalResult::Success(_));
        if expected != actual_bool {
            return None
        }
        Some(
            match actual {
                EvalResult::Success(mode)
                    | EvalResult::Failure(mode)
                => Case::new(Some(self), actual_bool)
                    .add_product(Product::new("mode", format!("{:#03o}", mode))),

                EvalResult::IoError(e) => Case::new(Some(self), false)
                    .add_product(Product::new("error", e)),
            }
        )
    }
}

impl Predicate<str> for FileModePredicate {
    fn eval(&self, variable: &str) -> bool {
        Predicate::<Path>::eval(self, variable.as_ref())
    }
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
struct ModeOctalFormatWrapper(u32);

trait ModeExt {
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

impl PredicateReflection for FileModePredicate {
    fn parameters<'a>(&'a self) -> Box<dyn Iterator<Item=Parameter<'a>> + 'a> {
        let params = vec![
            Parameter::new("required", self.required.octal_format_wrapper()),
            Parameter::new("prohibited", self.prohibited.octal_format_wrapper()),
        ];
        Box::new(params.into_iter())
    }
}

impl Display for FileModePredicate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "mode(required={:#03o}, prohibited={:#03o})",
            self.required,
            self.prohibited,
        )
    }
}

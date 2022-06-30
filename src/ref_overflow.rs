use std::fmt;

/// Error when trying to clone a [`Ref`], but there are already [`usize::MAX`]
/// references.
///
/// [`Ref`]: crate::Ref
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RefOverflow;

impl fmt::Display for RefOverflow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ref count exceeded `usize::MAX` ({}).", usize::MAX)
    }
}

impl std::error::Error for RefOverflow {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

use std::error::Error as StdError;
use std::fmt::{Debug, Display};

pub type MyResult<T> = Result<T, MyError>;

#[derive(thiserror::Error, Debug)]
pub enum MyError {
    #[error("{0}")]
    Generic(#[from] Box<dyn StdError>),

    #[error("failed to get wlr clipboard: {0}")]
    WlcrsPaste(#[from] wl_clipboard_rs::paste::Error),

    #[error("failed to set wlr clipboard: {0}")]
    WlcrsCopy(#[from] wl_clipboard_rs::copy::Error),

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    TerminalClipboard(#[from] StandardizedError<terminal_clipboard::ClipboardError>),

    #[error("{0}")]
    Arboard(#[from] arboard::Error),

    #[error("No clipboards.")]
    NoClipboards,
}

#[derive(Debug)]
pub struct StandardizedError<E: Debug> {
    pub inner: E,
}

impl<E: Debug> Display for StandardizedError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl<E: Debug> StdError for StandardizedError<E> {}

pub trait Standardize<T, E: Sized + Debug>: Sized {
    fn standardize(self) -> Result<T, StandardizedError<E>>;
}

impl<T, E: Sized + Debug> Standardize<T, E> for Result<T, E> {
    fn standardize(self) -> Result<T, StandardizedError<E>> {
        match self {
            Ok(ok) => Ok(ok),
            Err(err) => Err(StandardizedError { inner: err }),
        }
    }
}

pub trait Generify<T, E: 'static + StdError> {
    fn generify(self) -> Result<T, MyError>;
}

impl<T, E: 'static + StdError> Generify<T, E> for Result<T, E> {
    fn generify(self) -> Result<T, MyError> {
        match self {
            Ok(ok) => Ok(ok),
            Err(err) => Err(MyError::Generic(Box::new(err))),
        }
    }
}

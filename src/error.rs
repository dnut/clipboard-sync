use std::cell::{BorrowError, BorrowMutError};
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

    #[error("{0}")]
    BorrowError(#[from] BorrowError),

    #[error("{0}")]
    BorrowMutError(#[from] BorrowMutError),
}

#[derive(Debug)]
pub struct StandardizedError<E: Debug> {
    pub inner: E,
    pub stdio: Option<StdIo>,
}

impl<E: Debug> Display for StandardizedError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
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
            Err(err) => Err(StandardizedError { inner: err, stdio: None }),
        }
    }
}

impl<T, E: Sized + Debug> Standardize<T, E> for (Result<T, E>, StdIo) {
    fn standardize(self) -> Result<T, StandardizedError<E>> {
        match self.0 {
            Ok(ok) => Ok(ok),
            Err(err) => Err(StandardizedError { inner: err, stdio: Some(self.1) }),
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

macro_rules! stdio {
	($block:expr) => {{
		let mut stdout_redirect = gag::BufferRedirect::stdout().unwrap();
		let mut stderr_redirect = gag::BufferRedirect::stderr().unwrap();
		let ret = $block;
		let mut stdout = String::new();
		let mut stderr = String::new();
		stdout_redirect.read_to_string(&mut stdout).unwrap();
		stderr_redirect.read_to_string(&mut stderr).unwrap();
		(ret, crate::error::StdIo { stdout, stderr })
	}};
}
pub(crate) use stdio;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct StdIo {
	pub stdout: String,
	pub stderr: String,
}

impl StdIo {
    pub fn extend(&mut self, other: StdIo) {
        self.stdout = format!("{}{}", self.stdout, other.stdout);
        self.stderr = format!("{}{}", self.stderr, other.stderr);
    }
}

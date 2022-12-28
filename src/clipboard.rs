use async_trait::async_trait;
use futures::Future;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::{env, io::Read, process::Command, thread::sleep, time::Duration};
use terminal_clipboard::Clipboard as TerminalClipboard;
use wl_clipboard_rs::copy::{MimeType as CopyMimeType, Options, Source};
use wl_clipboard_rs::paste::{
    get_contents, ClipboardType, Error as PasteError, MimeType as PasteMimeType, Seat,
};

use crate::asyncification::asyncify;
use crate::error::{Generify, MyResult, Standardize};

#[async_trait(?Send)]
pub trait Clipboard: std::fmt::Debug {
    fn display(&self) -> String;
    async fn get(&self) -> MyResult<String>;
    async fn set(&self, value: &str) -> MyResult<()>;
    async fn watch(&self) -> MyResult<String> {
        let start = self.get().await?;
        loop {
            let now = self.get().await?;
            if now != start {
                return Ok(now);
            }
            sleep(Duration::from_millis(1000));
        }
    }
}

#[async_trait(?Send)]
impl<T: Clipboard> Clipboard for Box<T> {
    async fn get(&self) -> MyResult<String> {
        (**self).get().await
    }

    async fn set(&self, value: &str) -> MyResult<()> {
        (**self).set(value).await
    }

    fn display(&self) -> String {
        (**self).display()
    }
}

#[derive(Debug, Clone)]
pub struct WlrClipboard {
    pub display: String,
}

impl WlrClipboard {
    fn sync_get(&self) -> MyResult<String> {
        println!("asdasd");
        env::set_var("WAYLAND_DISPLAY", self.display.clone());
        let result = get_contents(
            ClipboardType::Regular,
            Seat::Unspecified,
            PasteMimeType::Text,
        );

        match result {
            Ok((mut pipe, _)) => {
                let mut contents = vec![];
                pipe.read_to_end(&mut contents)?;
                Ok(String::from_utf8_lossy(&contents).to_string())
            }

            Err(PasteError::NoSeats)
            | Err(PasteError::ClipboardEmpty)
            | Err(PasteError::NoMimeType) => Ok("".to_string()),

            Err(err) => Err(err)?,
        }
    }

    fn sync_set(&self, value: &str) -> MyResult<()> {
        env::set_var("WAYLAND_DISPLAY", self.display.clone());
        let opts = Options::new();
        let result = std::panic::catch_unwind(|| {
            opts.copy(
                Source::Bytes(value.to_string().into_bytes().into()),
                CopyMimeType::Text,
            )
        });

        Ok(result.standardize().generify()??)
    }
}

#[async_trait(?Send)]
impl Clipboard for WlrClipboard {
    fn display(&self) -> String {
        self.display.clone()
    }

    async fn get(&self) -> MyResult<String> {
        // let s = self.clone();
        // asyncify(move || s.sync_get()).await
        self.sync_get()
    }

    async fn set(&self, value: &str) -> MyResult<()> {
        // let s = self.clone();
        // let v = value.to_owned();
        // asyncify(move || s.sync_set(&v)).await
        self.sync_set(value)
    }
}

#[derive(Debug)]
pub struct CommandClipboard {
    display: String,
}

#[async_trait(?Send)]
impl Clipboard for CommandClipboard {
    fn display(&self) -> String {
        self.display.clone()
    }

    async fn get(&self) -> MyResult<String> {
        env::set_var("WAYLAND_DISPLAY", self.display.clone());
        let out = Command::new("wl-paste").output()?.stdout;
        Ok(String::from_utf8_lossy(&out).trim().to_string())
    }

    async fn set(&self, value: &str) -> MyResult<()> {
        env::set_var("WAYLAND_DISPLAY", self.display.clone());
        Command::new("wl-copy").arg(value).spawn()?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct ArClipboard {
    display: String,
}

#[async_trait(?Send)]
impl Clipboard for ArClipboard {
    fn display(&self) -> String {
        self.display.clone()
    }

    async fn get(&self) -> MyResult<String> {
        env::set_var("WAYLAND_DISPLAY", self.display.clone());
        let mut clipboard = arboard::Clipboard::new()?;
        Ok(clipboard.get_text().unwrap_or_default())
    }

    async fn set(&self, value: &str) -> MyResult<()> {
        env::set_var("WAYLAND_DISPLAY", self.display.clone());
        let mut clipboard = arboard::Clipboard::new()?;
        clipboard.set_text(value.into())?;

        Ok(())
    }
}

pub struct X11Clipboard {
    display: String,
    backend: X11Backend,
}

#[derive(Clone)]
pub struct X11Backend(Rc<RefCell<terminal_clipboard::X11Clipboard>>);
impl X11Backend {
    /// try to only call this once because repeated initializations may not work.
    /// i started seeing timeouts/errors after 4
    pub fn new() -> MyResult<Self> {
        // let backend = stdio! { terminal_clipboard::X11Clipboard::new() }.standardize()?;
        let backend = terminal_clipboard::X11Clipboard::new().unwrap();

        Ok(Self(Rc::new(RefCell::new(backend))))
    }
}

impl std::fmt::Debug for X11Clipboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("X11Clipboard")
            .field("display", &self.display)
            .finish()
    }
}

impl X11Clipboard {
    pub fn new(display: String, backend: X11Backend) -> Self {
        Self { display, backend }
    }
}

#[async_trait(?Send)]
impl Clipboard for X11Clipboard {
    fn display(&self) -> String {
        self.display.clone()
    }

    async fn get(&self) -> MyResult<String> {
        env::set_var("DISPLAY", self.display.clone());
        Ok(self
            .backend
            .0
            .try_borrow()?
            .get_string()
            .unwrap_or_default())
    }

    async fn set(&self, value: &str) -> MyResult<()> {
        env::set_var("DISPLAY", self.display.clone());
        self.backend
            .0
            .try_borrow_mut()?
            .set_string(value)
            .standardize()?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct HybridClipboard<G: Clipboard, S: Clipboard> {
    getter: G,
    setter: S,
}

// impl HybridClipboard<X11Clipboard, CommandClipboard> {
//     fn gnome(n: u8) -> MyResult<Self> {
//         Ok(Self {
//             getter: X11Clipboard::new(format!(":{}", n))?,
//             setter: CommandClipboard {
//                 display: format!("wayland-{}", n),
//             },
//         })
//     }
// }

#[async_trait(?Send)]
impl<G: Clipboard, S: Clipboard> Clipboard for HybridClipboard<G, S> {
    fn display(&self) -> String {
        self.getter.display()
    }

    async fn get(&self) -> MyResult<String> {
        self.getter.get().await
    }

    async fn set(&self, value: &str) -> MyResult<()> {
        self.setter.set(value).await
    }
}

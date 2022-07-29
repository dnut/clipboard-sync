use std::cell::RefCell;
use std::{env, io::Read, process::Command, thread::sleep, time::Duration};
use terminal_clipboard::Clipboard as TerminalClipboard;
use wl_clipboard_rs::copy::{MimeType as CopyMimeType, Options, Source};
use wl_clipboard_rs::paste::{
    get_contents, ClipboardType, Error as PasteError, MimeType as PasteMimeType, Seat,
};

use crate::error::{stdio, Generify, MyResult, Standardize};

pub trait Clipboard: std::fmt::Debug {
    fn display(&self) -> String;
    fn get(&self) -> MyResult<String>;
    fn set(&self, value: &str) -> MyResult<()>;
    fn watch(&self) -> MyResult<String> {
        let start = self.get()?;
        loop {
            let now = self.get()?;
            if now != start {
                return Ok(now);
            }
            sleep(Duration::from_millis(1000));
        }
    }
}

impl<T: Clipboard> Clipboard for Box<T> {
    fn get(&self) -> MyResult<String> {
        (**self).get()
    }

    fn set(&self, value: &str) -> MyResult<()> {
        (**self).set(value)
    }

    fn display(&self) -> String {
        (**self).display()
    }
}

#[derive(Debug)]
pub struct WlrClipboard {
    pub display: String,
}

impl Clipboard for WlrClipboard {
    fn display(&self) -> String {
        self.display.clone()
    }

    fn get(&self) -> MyResult<String> {
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

    fn set(&self, value: &str) -> MyResult<()> {
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

#[derive(Debug)]
pub struct CommandClipboard {
    display: String,
}

impl Clipboard for CommandClipboard {
    fn display(&self) -> String {
        self.display.clone()
    }

    fn get(&self) -> MyResult<String> {
        env::set_var("WAYLAND_DISPLAY", self.display.clone());
        let out = Command::new("wl-paste").output()?.stdout;
        Ok(String::from_utf8_lossy(&out).trim().to_string())
    }

    fn set(&self, value: &str) -> MyResult<()> {
        env::set_var("WAYLAND_DISPLAY", self.display.clone());
        Command::new("wl-copy").arg(value).spawn()?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct ArClipboard {
    display: String,
}

impl Clipboard for ArClipboard {
    fn display(&self) -> String {
        self.display.clone()
    }

    fn get(&self) -> MyResult<String> {
        env::set_var("WAYLAND_DISPLAY", self.display.clone());
        let mut clipboard = arboard::Clipboard::new()?;
        Ok(clipboard.get_text().unwrap_or("".to_string()))
    }

    fn set(&self, value: &str) -> MyResult<()> {
        env::set_var("WAYLAND_DISPLAY", self.display.clone());
        let mut clipboard = arboard::Clipboard::new()?;
        clipboard.set_text(value.into())?;

        Ok(())
    }
}

pub struct X11Clipboard {
    display: String,
    backend: RefCell<terminal_clipboard::X11Clipboard>,
}

impl std::fmt::Debug for X11Clipboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("X11Clipboard")
            .field("display", &self.display)
            .finish()
    }
}

impl X11Clipboard {
    pub fn new(display: String) -> MyResult<Self> {
        let cb = stdio! { terminal_clipboard::X11Clipboard::new() }.standardize()?;

        Ok(Self {
            display,
            backend: RefCell::new(cb),
        })
    }
}

impl Clipboard for X11Clipboard {
    fn display(&self) -> String {
        self.display.clone()
    }

    fn get(&self) -> MyResult<String> {
        env::set_var("DISPLAY", self.display.clone());
        Ok(self
            .backend
            .try_borrow()?
            .get_string()
            .unwrap_or("".to_string()))
    }

    fn set(&self, value: &str) -> MyResult<()> {
        env::set_var("DISPLAY", self.display.clone());
        self.backend
            .try_borrow_mut()?
            .set_string(value.into())
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

impl<G: Clipboard, S: Clipboard> Clipboard for HybridClipboard<G, S> {
    fn display(&self) -> String {
        self.getter.display()
    }

    fn get(&self) -> MyResult<String> {
        self.getter.get()
    }

    fn set(&self, value: &str) -> MyResult<()> {
        self.setter.set(value)
    }
}

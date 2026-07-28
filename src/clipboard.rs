use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{env, process::Command, thread};

use chrono::Local;

use crate::error::MyResult;
use crate::log;
use crate::wlr_backend::WlrBackend;

pub trait Clipboard: std::fmt::Debug {
    fn display(&self) -> String;

    fn get(&self) -> MyResult<String>;

    fn set(&self, value: &str) -> MyResult<()>;

    fn should_poll(&self) -> bool {
        true
    }

    fn rank(&self) -> u8 {
        100
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

    fn should_poll(&self) -> bool {
        (**self).should_poll()
    }

    fn rank(&self) -> u8 {
        (**self).rank()
    }
}

pub struct WlrClipboard {
    pub display: String,
    backend: WlrBackend,
}

impl std::fmt::Debug for WlrClipboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WlrClipboard")
            .field("display", &self.display)
            .finish()
    }
}

impl WlrClipboard {
    pub fn new(display: String) -> MyResult<Self> {
        env::set_var("WAYLAND_DISPLAY", &display);
        Ok(Self {
            backend: WlrBackend::new(&display)?,
            display,
        })
    }
}

impl Clipboard for WlrClipboard {
    fn display(&self) -> String {
        self.display.clone()
    }

    fn get(&self) -> MyResult<String> {
        self.backend.get_text_or_empty()
    }

    fn set(&self, value: &str) -> MyResult<()> {
        self.backend.set_text_result(value)
    }

    fn rank(&self) -> u8 {
        10
    }
}

#[derive(Debug)]
pub struct WlCommandClipboard {
    pub display: String,
}

impl Clipboard for WlCommandClipboard {
    fn display(&self) -> String {
        self.display.clone()
    }

    fn get(&self) -> MyResult<String> {
        let out = Command::new("wl-paste")
            .env("WAYLAND_DISPLAY", &self.display)
            .output()?
            .stdout;
        Ok(String::from_utf8_lossy(&out).trim().to_string())
    }

    fn set(&self, value: &str) -> MyResult<()> {
        Command::new("wl-copy")
            .arg(value)
            .env("WAYLAND_DISPLAY", &self.display)
            .spawn()?;
        Ok(())
    }

    fn should_poll(&self) -> bool {
        false
    }

    fn rank(&self) -> u8 {
        200
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
        Ok(clipboard.get_text().unwrap_or_default())
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
    setter: x11_clipboard::Clipboard,
    cache: Arc<Mutex<String>>,
}

impl std::fmt::Debug for X11Clipboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("X11Clipboard")
            .field("display", &self.display)
            .finish()
    }
}

/// Bound on a single X11 read, used both for the startup seed and by the
/// watcher thread. Slow INCR transfers must not block reads indefinitely.
const X11_READ_TIMEOUT: Duration = Duration::from_secs(30);

impl X11Clipboard {
    pub fn new(display: String) -> MyResult<Self> {
        env::set_var("DISPLAY", &display);
        let setter = x11_clipboard::Clipboard::new()?;
        let watcher = x11_clipboard::Clipboard::new()?;

        let atoms = &watcher.getter.atoms;
        let initial = watcher
            .load(
                atoms.clipboard,
                atoms.utf8_string,
                atoms.property,
                X11_READ_TIMEOUT,
            )
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default();

        let cache = Arc::new(Mutex::new(initial));
        thread::spawn({
            let cache = cache.clone();
            move || watch_x11(watcher, cache)
        });

        Ok(Self {
            display,
            setter,
            cache,
        })
    }
}

/// Polls the X11 clipboard in a dedicated thread so the main loop never blocks
/// on a transfer. Selection owners that proxy clipboard data from another
/// machine serve INCR transfers slowly; done synchronously in the poll loop,
/// that starves Wayland event dispatch and pastes from Wayland apps time out.
/// The XFixes-driven load_wait is not used: it has no read timeout, and rapid
/// ownership changes (our own set racing another client re-taking the
/// selection) make its shared-property conversions stomp each other — a stuck
/// transfer would freeze this watcher silently. Bounded reads retry, so no
/// change is ever lost.
fn watch_x11(cb: x11_clipboard::Clipboard, cache: Arc<Mutex<String>>) {
    let atoms = &cb.getter.atoms;
    loop {
        match cb.load(atoms.clipboard, atoms.utf8_string, atoms.property, X11_READ_TIMEOUT) {
            Ok(bytes) => *cache.lock().unwrap() = String::from_utf8_lossy(&bytes).into_owned(),
            Err(e) => log::debug!("X11 clipboard watch error: {e}"),
        }
        thread::sleep(Duration::from_millis(500));
    }
}

impl Clipboard for X11Clipboard {
    fn display(&self) -> String {
        self.display.clone()
    }

    fn get(&self) -> MyResult<String> {
        Ok(self.cache.lock().unwrap().clone())
    }

    fn set(&self, value: &str) -> MyResult<()> {
        let atoms = &self.setter.setter.atoms;
        self.setter.store(atoms.clipboard, atoms.utf8_string, value)?;
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

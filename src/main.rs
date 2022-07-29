use chrono::Local;
use error::{Generify, MyError, MyResult, Standardize, StdIo};
use itertools::Itertools;
use nix::sys::signal::{self, Signal};
use nix::sys::wait::{WaitPidFlag, WaitStatus};
use nix::unistd::{fork, Pid};
use nix::{sys::wait::waitpid, unistd::ForkResult};
use std::cell::RefCell;
use std::thread;
use std::time::SystemTime;
use std::{env, io::Read, process::Command, thread::sleep, time::Duration};
use terminal_clipboard::Clipboard as TerminalClipboard;
use wayland_client::ConnectError;
use wl_clipboard_rs::copy::{MimeType as CopyMimeType, Options, Source};
use wl_clipboard_rs::paste::{
    get_contents, ClipboardType, Error as PasteError, MimeType as PasteMimeType, Seat,
};

use crate::error::stdio;

mod error;
mod log;

fn main() {
    let mut c = 0;
    loop {
        match unsafe { fork() }.expect("Failed to fork") {
            ForkResult::Parent { child } => {
                if c == 0 {
                    log::info!("started clipboard sync manager");
                }
                kill_after(child, 600);
                let status = waitpid(Some(child), None);
                log::info!("child process completed with: {:?}", status);
                sleep(Duration::from_secs(1));
                c += 1;
            }
            ForkResult::Child => run(),
        }
    }
}

pub fn kill_after(pid: Pid, seconds: u64) {
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(seconds));
        if let Ok(WaitStatus::StillAlive) = waitpid(Some(pid), Some(WaitPidFlag::WNOWAIT)) {
            log::info!("killing subprocess {pid}");
            signal::kill(pid, Signal::SIGTERM).unwrap();
        }
    });
}

fn run() {
    log::info!("starting clipboard sync");
    loop_with_error_pain_management(
        get_clipboards().unwrap(), //
        |cb| keep_synced(cb),
        |_| get_clipboards().unwrap(),
    )
    .unwrap();
}

/// Execute an action with a sophistocated retry mechanism
/// If the action fails:
/// - 1. run a recovery step to manipulate the input
/// - 2. attempt to execute the action again
/// If the action fails too frequently, exit
fn loop_with_error_pain_management<
    Input,
    Return,
    Action: Fn(&Input) -> MyResult<Return>,
    Recovery: Fn(Input) -> Input,
>(
    // data passed into action and reset by recovery
    initial_input: Input,
    // action to attempt on every iteration
    action: Action,
    // action to attempt on every error - errors here are not yet handled. you can panic if necessary
    recovery: Recovery,
) -> MyResult<Return> {
    let mut input = initial_input;
    let mut errorcount: u64 = 0;
    let mut first_error: SystemTime = SystemTime::UNIX_EPOCH;
    let mut last_error: SystemTime = SystemTime::UNIX_EPOCH;
    loop {
        match action(&input) {
            Ok(ret) => return Ok(ret),
            Err(err) => {
                log::error!("action exited with error: {:?}", err);
                let now = SystemTime::now();
                input = recovery(input);
                if errorcount == 0 {
                    first_error = now;
                } else {
                    if SystemTime::now().duration_since(last_error).unwrap()
                        > Duration::from_secs(10)
                    {
                        errorcount = 0;
                    } else {
                        let error_session_seconds = SystemTime::now()
                            .duration_since(first_error)
                            .unwrap()
                            .as_secs();
                        let error_session_rate_scaled = errorcount
                            .checked_mul(100)
                            .unwrap_or(u64::MAX)
                            .checked_div(error_session_seconds)
                            .unwrap_or(u64::MAX);
                        let error_pain = error_session_rate_scaled
                            .checked_add(error_session_seconds)
                            .unwrap_or(u64::MAX);
                        if error_pain > 100 {
                            Err(format!("too many errors, exiting"))
                                .standardize()
                                .generify()?;
                        }
                    }
                }
                last_error = now;
                errorcount += 1;
                sleep(Duration::from_millis(1000));
            }
        }
        log::info!("retrying");
    }
}

fn get_clipboards() -> MyResult<Vec<Box<dyn Clipboard>>> {
    let mut clipboards = get_clipboards_spec(get_wayland);
    clipboards.extend(get_clipboards_spec(get_x11));

    let mut remove_me = vec![];
    for combo in clipboards.iter().enumerate().combinations(2) {
        let (_, cb1) = combo[0];
        let (i2, cb2) = combo[1];
        if are_same(cb1, cb2)? {
            log::debug!("{cb1:?} is the same as {cb2:?}, removing {cb2:?}");
            remove_me.push(i2);
        }
    }
    let clipboards = clipboards
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !remove_me.contains(i))
        .map(|(_, c)| c)
        .collect::<Vec<Box<dyn Clipboard>>>();

    log::info!("Using clipboards: {:?}", clipboards);

    Ok(clipboards)
}

fn are_same(one: &Box<dyn Clipboard>, two: &Box<dyn Clipboard>) -> MyResult<bool> {
    let d1 = &one.display();
    let d2 = &two.display();
    one.set(d1)?;
    if d1 != &two.get()? {
        return Ok(false);
    }
    two.set(d2)?;
    if d2 != &one.get()? {
        return Ok(false);
    }

    Ok(true)
}

enum OptionIo<T> {
    Some(T),
    None,
    StdIo(StdIo),
}

fn get_clipboards_spec<F: Fn(u8) -> MyResult<OptionIo<Box<dyn Clipboard>>>>(
    getter: F,
) -> Vec<Box<dyn Clipboard>> {
    let mut clipboards: Vec<Box<dyn Clipboard>> = Vec::new();
    let mut combined_stdio = StdIo::default();
    for i in 0..u8::MAX {
        let result = getter(i);
        match result {
            Ok(option) => match option {
                OptionIo::Some(clipboard) => {
                    log::debug!("Found clipboard: {:?}", clipboard);
                    clipboards.push(clipboard);
                }
                OptionIo::None => (),
                OptionIo::StdIo(stdio) => combined_stdio.extend(stdio),
            },
            Err(err) => log::error!(
                "unexpected error while attempting to setup clipboard {}: {}",
                i,
                err
            ),
        }
    }
    if combined_stdio != Default::default() {
        log::truncate_to_debug!(log::error, "Got some unexpected output while locating clipboards, maybe you need to execute `xhost +localhost` in your x11 environments?: {:?}", combined_stdio);
    }

    clipboards
}

fn get_wayland(n: u8) -> MyResult<OptionIo<Box<dyn Clipboard>>> {
    let wl_display = format!("wayland-{}", n);
    let clipboard = WlrClipboard {
        display: wl_display.clone(),
    };
    let attempt = clipboard.get();
    if let Err(MyError::WlcrsPaste(PasteError::WaylandConnection(
        ConnectError::NoCompositorListening,
    ))) = attempt
    {
        return Ok(OptionIo::None);
    }
    if let Err(MyError::WlcrsPaste(PasteError::MissingProtocol {
        name: "zwlr_data_control_manager_v1",
        version: 1,
    })) = attempt
    {
        log::error!("{wl_display} does not support zwlr_data_control_manager_v1, trying with X11Clipboard...");
        return Ok(OptionIo::None);
    }
    attempt?;

    Ok(OptionIo::Some(Box::new(clipboard)))
}

fn get_x11(n: u8) -> MyResult<OptionIo<Box<dyn Clipboard>>> {
    let display = format!(":{}", n);
    let clipboard = X11Clipboard::new(display);
    match clipboard {
        Ok(clipboard) => {
            clipboard.get()?;
            Ok(OptionIo::Some(Box::new(clipboard)))
        }
        Err(MyError::TerminalClipboard(e)) => Ok(OptionIo::StdIo(e.stdio.unwrap_or_default())),
        Err(e) => Err(e),
    }
}

fn keep_synced(clipboards: &Vec<Box<dyn Clipboard>>) -> MyResult<()> {
    if clipboards.len() == 0 {
        return Err(MyError::NoClipboards);
    }
    let start = clipboards
        .iter()
        .map(|c| c.get().unwrap_or("".to_string()))
        .find(|s| s != "")
        .unwrap_or("".to_string());
    for c in clipboards {
        c.set(&start)?;
    }
    loop {
        sleep(Duration::from_millis(100));
        let new_value = await_change(clipboards)?;
        for c in clipboards {
            c.set(&new_value)?;
        }
    }
}

fn await_change(clipboards: &Vec<Box<dyn Clipboard>>) -> MyResult<String> {
    let start = clipboards[0].get()?;
    loop {
        for c in clipboards {
            let new = c.get()?;
            if new != start {
                log::info!("clipboard updated from display {}", c.display());
                return Ok(new);
            }
        }
        sleep(Duration::from_millis(200));
    }
}

trait Clipboard: std::fmt::Debug {
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
struct WlrClipboard {
    display: String,
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
struct CommandClipboard {
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
struct ArClipboard {
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

struct X11Clipboard {
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
    fn new(display: String) -> MyResult<Self> {
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
struct HybridClipboard<G: Clipboard, S: Clipboard> {
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

// #[test]
// fn test() {
//     log::info!("{:?}", get_clipboard(0).unwrap());
//     log::info!("{:?}", get_clipboard(1).unwrap());
//     log::info!("{:?}", get_clipboard(2).unwrap());
// }

use error::{Generify, MyError, MyResult, Standardize};
use std::time::SystemTime;
use std::{env, io::Read, process::Command, thread::sleep, time::Duration};
use terminal_clipboard::Clipboard as TerminalClipboard;
use wayland_client::ConnectError;
use wl_clipboard_rs::copy::{MimeType as CopyMimeType, Options, Source};
use wl_clipboard_rs::paste::{
    get_contents, ClipboardType, Error as PasteError, MimeType as PasteMimeType, Seat,
};

mod error;
// mod zombies;

fn main() {
    println!("starting clipboard sync");
    loop_with_error_pain_management(
        get_clipboards(), //
        |cb| keep_synced(cb),
        |_| get_clipboards(),
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
                eprintln!("keep_synced exited with error: {:?}", err);
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
    }
}

fn get_clipboards() -> Vec<Box<dyn Clipboard>> {
    let mut clipboards: Vec<Box<dyn Clipboard>> = Vec::new();
    for i in 0..u8::MAX {
        let result = get_clipboard(i);
        match result {
            Ok(option) => match option {
                Some(clipboard) => {
                    println!("Using clipboard: {:?}", clipboard);
                    clipboards.push(clipboard);
                }
                None => (),
            },
            Err(err) => eprintln!(
                "unexpected error while attempting to setup clipboard {}: {}",
                i, err
            ),
        }
    }

    clipboards
}

fn get_clipboard(n: u8) -> MyResult<Option<Box<dyn Clipboard>>> {
    let wl_display = format!("wayland-{}", n);
    let clipboard = WlrClipboard {
        display: wl_display.clone(),
    };
    let attempt = clipboard.get();
    if let Err(MyError::WlcrsPaste(PasteError::WaylandConnection(
        ConnectError::NoCompositorListening,
    ))) = attempt
    {
        return Ok(None);
    }
    if let Err(MyError::WlcrsPaste(PasteError::MissingProtocol {
        name: "zwlr_data_control_manager_v1",
        version: 1,
    })) = attempt
    {
        return Ok(Some(Box::new(HybridClipboard {
            display: format!(":{}", n),
            wayland_display: wl_display,
            backend: terminal_clipboard::X11Clipboard::new().unwrap(),
        })));
    }
    attempt?;

    Ok(Some(Box::new(clipboard)))
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
                println!("{}: old '{}' new '{}' ", c.display(), start, new);
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

#[derive(Debug)]
struct X11Clipboard {
    display: String,
}

impl Clipboard for X11Clipboard {
    fn display(&self) -> String {
        self.display.clone()
    }

    fn get(&self) -> MyResult<String> {
        env::set_var("DISPLAY", self.display.clone());
        let clipboard = terminal_clipboard::X11Clipboard::new().standardize()?;
        Ok(clipboard.get_string().unwrap_or("".to_string()))
    }

    fn set(&self, value: &str) -> MyResult<()> {
        env::set_var("DISPLAY", self.display.clone());
        let mut clipboard = terminal_clipboard::X11Clipboard::new().standardize()?;
        clipboard.set_string(value.into()).standardize()?;

        Ok(())
    }
}

struct HybridClipboard {
    display: String,
    wayland_display: String,
    backend: terminal_clipboard::X11Clipboard,
}

impl std::fmt::Debug for HybridClipboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HybridClipboard")
            .field("display", &self.display)
            .field("wayland_display", &self.wayland_display)
            .finish()
    }
}

impl Clipboard for HybridClipboard {
    fn display(&self) -> String {
        self.display.clone()
    }

    fn get(&self) -> MyResult<String> {
        env::set_var("DISPLAY", self.display.clone());
        // let clipboard = terminal_clipboard::X11Clipboard::new().unwrap();
        Ok(self.backend.get_string().unwrap_or("".to_string()))
    }

    fn set(&self, value: &str) -> MyResult<()> {
        env::set_var("WAYLAND_DISPLAY", self.wayland_display.clone());
        Command::new("wl-copy").arg(value).spawn()?;

        Ok(())
    }
}

#[test]
fn test() {
    println!("{:?}", get_clipboard(0).unwrap());
    println!("{:?}", get_clipboard(1).unwrap());
    println!("{:?}", get_clipboard(2).unwrap());
}

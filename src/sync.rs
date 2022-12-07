use chrono::Local;
use itertools::Itertools;
use std::{thread::sleep, time::Duration};
use wayland_client::ConnectError;
use wl_clipboard_rs::paste::Error as PasteError;

use crate::clipboard::*;
use crate::error::{MyError, MyResult, StdIo};
use crate::log;

pub fn get_clipboards() -> MyResult<Vec<Box<dyn Clipboard>>> {
    let mut clipboards = get_clipboards_spec(get_wayland);
    clipboards.extend(get_clipboards_spec(get_x11));

    let start = clipboards
        .iter()
        .map(|c| c.get().unwrap_or("".to_string()))
        .find(|s| s != "")
        .unwrap_or("".to_string());

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

    for c in clipboards.iter() {
        c.set(&start)?;
    }

    log::info!("Using clipboards: {:?}", clipboards);

    Ok(clipboards)
}

pub fn keep_synced(clipboards: &Vec<Box<dyn Clipboard>>) -> MyResult<()> {
    if clipboards.len() == 0 {
        return Err(MyError::NoClipboards);
    }
    loop {
        sleep(Duration::from_millis(100));
        let new_value = await_change(clipboards)?;
        for c in clipboards {
            c.set(&new_value)?;
        }
    }
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

use chrono::Local;
use std::collections::HashSet;
use std::thread::sleep;
use std::time::{Duration, Instant};
use wayland_client::ConnectError;

use crate::wlr_backend::Error as WlrBackendError;

use crate::clipboard::*;
use crate::error::{MyError, MyResult};
use crate::log::{self, concise_numbers};

pub fn get_clipboards() -> MyResult<Vec<Box<dyn Clipboard>>> {
    log::debug!("identifying unique clipboards...");
    let mut clipboards = get_clipboards_spec(get_wayland);
    // let x11_backend = X11Backend::new()?;
    clipboards.extend(get_clipboards_spec(get_x11));

    let start = clipboards
        .iter()
        .map(|c| c.get().unwrap_or_default())
        .find(|s| !s.is_empty())
        .unwrap_or_default();
    log::sensitive!(log::info, "Clipboard contents at the start: '{start}'");

    let mut remove_me = HashSet::new();
    let len = clipboards.len();
    for i in 0..clipboards.len() {
        if !remove_me.contains(&i) {
            let cb1 = &clipboards[i];
            for (j, cb2) in clipboards.iter().enumerate().take(len).skip(i + 1) {
                if are_same(&**cb1, &**cb2)? {
                    if cb1.rank() < cb2.rank() {
                        log::debug!("dupe detected: {cb1:?} == {cb2:?} -> removing {cb2:?}");
                        remove_me.insert(j);
                    } else {
                        log::debug!("dupe detected: {cb1:?} == {cb2:?} -> removing {cb1:?}");
                        remove_me.insert(i);
                    }
                }
            }
        }
    }

    let clipboards = clipboards
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !remove_me.contains(i))
        .map(|(_, c)| c)
        .collect::<Vec<Box<dyn Clipboard>>>();

    // let clipboards = dedupe(clipboards)?;

    for c in clipboards.iter() {
        c.set(&start)?;
    }

    log::info!("Using clipboards: {:?}", clipboards);

    Ok(clipboards)
}

pub fn keep_synced(clipboards: &Vec<Box<dyn Clipboard>>) -> MyResult<()> {
    if clipboards.is_empty() {
        return Err(MyError::NoClipboards);
    }

    let mut last: Vec<String> = clipboards
        .iter()
        .map(|c| c.get().unwrap_or_default())
        .collect();
    let mut settle_until: Option<Instant> = None;

    loop {
        sleep(Duration::from_millis(200));

        if let Some(deadline) = settle_until {
            if Instant::now() < deadline {
                continue;
            }
            for (j, c) in clipboards.iter().enumerate() {
                if c.should_poll() {
                    last[j] = c.get().unwrap_or_default();
                }
            }
            settle_until = None;
        }

        for (i, c) in clipboards.iter().enumerate() {
            if !c.should_poll() {
                continue;
            }
            let current = c.get()?;
            // Read glitches (wayland NoSeats/NoMimeType when the selection holds
            // non-text data, X11 errors via unwrap_or_default) surface as "".
            // Propagating one would wipe every synced clipboard.
            if current.is_empty() || current == last[i] {
                continue;
            }

            log::info!("clipboard updated from display {}", c.display());
            log::sensitive!(log::info, "clipboard contents: '{}'", current);

            for (j, other) in clipboards.iter().enumerate() {
                if i == j {
                    continue;
                }
                if last[j] != current {
                    other.set(&current)?;
                }
                last[j] = current.clone();
            }
            last[i] = current;

            // One poll interval for Wayland/X11 writes to settle before re-baselining.
            settle_until = Some(Instant::now() + Duration::from_millis(200));
            break;
        }
    }
}

fn are_same(one: &dyn Clipboard, two: &dyn Clipboard) -> MyResult<bool> {
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

// /// equality comparison is the bottleneck, and it's composed of two steps. so
// /// the purpose of this function is to minimize the number of executions of
// /// those steps. to do so, these steps are run at different times and combined
// /// at the end. this requires some extra computation in here but that's fine.
// fn dedupe(clipboards: Vec<Box<dyn Clipboard>>) -> MyResult<Vec<Box<dyn Clipboard>>> {
//     let mut k_cant_read_v: HashMap<usize, HashSet<usize>> = HashMap::new();
//     let mut k_can_read_v: HashMap<usize, HashSet<usize>> = HashMap::new();
//     for (i, one) in clipboards.iter().enumerate() {
//         // let i_can_read = k_can_read_v.entry(i).or_insert(HashSet::new()).clone();
//         let i_cant_read = k_cant_read_v.entry(i).or_insert(HashSet::new()).clone();
//         let d1 = &one.display();
//         one.set(d1)?;
//         for (j, two) in clipboards.iter().enumerate() {
//             println!("{i} {j} {} {}", i != j, !i_cant_read.contains(&j));
//             if i != j && !i_cant_read.contains(&j) {
//                 if d1 == &two.get()? {
//                     log::debug!("{two:?} can read {one:?}");
//                     k_can_read_v.entry(j).or_insert(HashSet::new()).insert(i);
//                 } else {
//                     log::debug!("{two:?} cannot read {one:?}");
//                     k_cant_read_v.entry(j).or_insert(HashSet::new()).insert(i);
//                 }
//             }
//         }
//     }
//     let mut remove_me: HashSet<usize> = HashSet::new();
//     let mut dont_remove_me: HashSet<usize> = HashSet::new();
//     println!("{k_cant_read_v:#?}");
//     println!("{k_can_read_v:#?}");
//     for (k, v) in k_can_read_v.iter() {
//         for readable in v {
//             if !dont_remove_me.contains(readable)
//                 && k_can_read_v
//                     .get(readable)
//                     .map(|what_readable_can_read| what_readable_can_read.contains(k))
//                     .unwrap_or(false)
//             {
//                 remove_me.insert(*readable);
//                 dont_remove_me.insert(*k);
//             }
//         }
//     }

//     Ok(clipboards
//         .into_iter()
//         .enumerate()
//         .filter(|(i, _)| !remove_me.contains(i))
//         .map(|(_, c)| c)
//         .collect::<Vec<Box<dyn Clipboard>>>())
// }

// /// equality comparison is the bottleneck, and it's composed of two steps. so
// /// the purpose of this function is to minimize the number of executions of
// /// those steps. to do so, these steps are run at different times and combined
// /// at the end. this requires some extra computation in here but that's fine.
// fn dedupe(clipboards: Vec<Box<dyn Clipboard>>) -> MyResult<Vec<Box<dyn Clipboard>>> {
//     for (i, clipboard) in clipboards.iter().enumerate() {
//         println!("write {i}");
//         clipboard.set(&format!("{}{i}", clipboard.get()?))?;
//     }
//     let mut results = HashMap::new();
//     for (i, clipboard) in clipboards.iter().enumerate() {
//         println!("read {i}");
//         results.insert(i, clipboard.get()?);
//     }

//     todo!()
// }

fn get_clipboards_spec<F: Fn(u8) -> MyResult<Option<Box<dyn Clipboard>>>>(
    getter: F,
) -> Vec<Box<dyn Clipboard>> {
    let mut clipboards: Vec<Box<dyn Clipboard>> = Vec::new();
    let mut xcb_conn_err = None;
    let mut xcb_conn_failed_clipboards = vec![];
    for i in 0..u8::MAX {
        let result = getter(i);
        match result {
            Ok(option) => {
                if let Some(clipboard) = option {
                    log::debug!("Found clipboard: {:?}", clipboard);
                    clipboards.push(clipboard);
                }
            }
            Err(MyError::X11Clipboard(x11_clipboard::error::Error::XcbConn(err))) => {
                xcb_conn_failed_clipboards.push(i);
                xcb_conn_err = Some(err);
            },
            Err(err) => log::error!(
                "unexpected error while attempting to setup clipboard {}: {}",
                i,
                err
            ),
        }
    }
    if let Some(err) = xcb_conn_err {
        let displays = concise_numbers(&xcb_conn_failed_clipboards);
        log::warning!(
            "Issue connecting to some x11 clipboards. \
This is expected when hooking up to gnome wayland, and not a problem in that context. \
Details: '{err}' for x11 displays: {displays}",
        );
    }

    clipboards
}

fn get_wayland(n: u8) -> MyResult<Option<Box<dyn Clipboard>>> {
    let wl_display = format!("wayland-{}", n);
    match WlrClipboard::new(wl_display.clone()) {
        Ok(clipboard) => Ok(Some(Box::new(clipboard))),
        Err(MyError::WlrBackend(WlrBackendError::WaylandConnection(
            ConnectError::NoCompositor,
        )))
        | Err(MyError::WlrBackend(WlrBackendError::SocketOpenError(_))) => Ok(None),
        Err(MyError::WlrBackend(WlrBackendError::MissingProtocol { .. })) => {
            log::warning!(
                "{wl_display} does not support ext-data-control or wlr-data-control. If you are \
running gnome in wayland, that's OK because it provides an x11 clipboard, which will be used \
instead. Otherwise, `wl-copy` will be used to sync data *into* this clipboard, but it will not \
be possible to read data *from* this clipboard into other clipboards."
            );
            let command = WlCommandClipboard {
                display: wl_display.clone(),
            };
            let Ok(gotten) = command.get() else {
                return Ok(None);
            };
            let Ok(_) = command.set(&gotten) else {
                return Ok(None);
            };
            Ok(Some(Box::new(command)))
        }
        Err(err) => {
            log::error!(
                "unexpected error while attempting to setup wayland clipboard {wl_display}: {err}"
            );
            Ok(None)
        }
    }
}

fn get_x11(n: u8) -> MyResult<Option<Box<dyn Clipboard>>> {
    let display = format!(":{}", n);
    let clipboard = X11Clipboard::new(display)?;
    clipboard.get()?;

    Ok(Some(Box::new(clipboard)))
}


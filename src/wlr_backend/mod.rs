mod data_control;
mod seat_data;

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, Cursor, Read};
use std::os::fd::AsFd;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::thread;
use std::{env, mem};

use os_pipe::pipe;
use rustix::fs::{fcntl_setfl, OFlags};
use wayland_client::globals::{registry_queue_init, GlobalError, GlobalListContents};
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_seat::{self, WlSeat};
use wayland_client::{
    delegate_dispatch, event_created_child, ConnectError, Connection, Dispatch, DispatchError,
    EventQueue, Proxy,
};
use wayland_protocols::ext::data_control::v1::client::{
    ext_data_control_device_v1::ExtDataControlDeviceV1,
    ext_data_control_manager_v1::ExtDataControlManagerV1,
    ext_data_control_offer_v1::ExtDataControlOfferV1,
    ext_data_control_source_v1::ExtDataControlSourceV1,
};
use wayland_protocols_wlr::data_control::v1::client::{
    zwlr_data_control_device_v1::ZwlrDataControlDeviceV1,
    zwlr_data_control_manager_v1::ZwlrDataControlManagerV1,
    zwlr_data_control_offer_v1::ZwlrDataControlOfferV1,
    zwlr_data_control_source_v1::ZwlrDataControlSourceV1,
};
use wl_clipboard_rs::utils::is_text;

use crate::error::MyResult;
use data_control::{
    impl_dispatch_device, impl_dispatch_manager, impl_dispatch_offer, impl_dispatch_source,
    Manager, Offer, Source,
};
use seat_data::SeatData;

const TEXT_MIMES: &[&str] = &[
    "text/plain;charset=utf-8",
    "UTF8_STRING",
    "text/plain",
    "STRING",
    "TEXT",
];

pub struct ConnectionState {
    pub seats: HashMap<WlSeat, SeatData>,
    pub clipboard_manager: Manager,
}

struct SessionState {
    common: ConnectionState,
    offers: HashMap<Offer, Vec<String>>,
    data_sources: HashMap<String, Arc<[u8]>>,
    owned_sources: Vec<Source>,
}

struct WlrSession {
    queue: EventQueue<SessionState>,
    state: SessionState,
}

#[derive(Clone)]
pub struct WlrBackend(Rc<RefCell<WlrSession>>);

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("There are no seats")]
    NoSeats,

    #[error("The clipboard is empty")]
    ClipboardEmpty,

    #[error("No suitable type of content copied")]
    NoMimeType,

    #[error("Couldn't open the provided Wayland socket")]
    SocketOpenError(#[source] io::Error),

    #[error("Couldn't connect to the Wayland compositor")]
    WaylandConnection(#[source] ConnectError),

    #[error("Wayland compositor communication error")]
    WaylandCommunication(#[source] DispatchError),

    #[error(
        "A required Wayland protocol ({name} version {version}) is not supported by the compositor"
    )]
    MissingProtocol { name: &'static str, version: u32 },

    #[error("Couldn't create a pipe for content transfer")]
    PipeCreation(#[source] io::Error),

    #[error("Clipboard I/O error")]
    Io(#[source] io::Error),
}

impl AsMut<ConnectionState> for SessionState {
    fn as_mut(&mut self) -> &mut ConnectionState {
        &mut self.common
    }
}

delegate_dispatch!(SessionState: [WlSeat: ()] => ConnectionState);

impl Dispatch<WlRegistry, GlobalListContents> for SessionState {
    fn event(
        _state: &mut Self,
        _proxy: &WlRegistry,
        _event: <WlRegistry as wayland_client::Proxy>::Event,
        _data: &GlobalListContents,
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

impl_dispatch_manager!(SessionState);

impl_dispatch_device!(SessionState, WlSeat, |state: &mut SessionState, event, seat| {
    match event {
        Event::DataOffer { id } => {
            state.offers.entry(Offer::from(id)).or_default();
        }
        Event::Selection { id } => {
            let offer = id.map(Offer::from);
            state.common.seats.get_mut(seat).unwrap().set_offer(offer);
        }
        Event::Finished => {
            state.common.seats.get_mut(seat).unwrap().set_device(None);
        }
        _ => (),
    }
});

impl_dispatch_offer!(SessionState, |state: &mut SessionState, offer: Offer, event| {
    if let Event::Offer { mime_type } = event {
        state.offers.entry(offer).or_default().push(mime_type);
    }
});

impl_dispatch_source!(SessionState, |state: &mut SessionState,
                                     source: Source,
                                     event| {
    match event {
        Event::Send { mime_type, fd } => {
            if let Some(data) = state.data_sources.get(&mime_type) {
                // Serve from a side thread: a blocking write larger than the
                // pipe capacity would deadlock the dispatch loop when the
                // reader only starts draining after this handler returns —
                // which is exactly what our own get_text() roundtrip does.
                let data = data.clone();
                thread::spawn(move || {
                    let _ = (|| -> io::Result<()> {
                        fcntl_setfl(&fd, OFlags::empty())?;
                        let mut target = File::from(fd);
                        io::copy(&mut Cursor::new(&*data), &mut target).map(drop)
                    })();
                });
            }
        }
        Event::Cancelled => source.destroy(),
        _ => (),
    }
});

impl<S> Dispatch<WlSeat, (), S> for ConnectionState
where
    S: Dispatch<WlSeat, ()> + AsMut<ConnectionState>,
{
    fn event(
        parent: &mut S,
        seat: &WlSeat,
        event: <WlSeat as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qh: &wayland_client::QueueHandle<S>,
    ) {
        if let wl_seat::Event::Name { name } = event {
            parent
                .as_mut()
                .seats
                .get_mut(seat)
                .unwrap()
                .set_name(name);
        }
    }
}

impl WlrBackend {
    pub fn new(display: &str) -> Result<Self, Error> {
        let socket_name = OsString::from(display);
        let (mut queue, mut common) = connect(false, Some(socket_name))?;

        if common.seats.is_empty() {
            return Err(Error::NoSeats);
        }

        for (seat, data) in &mut common.seats {
            let device = common
                .clipboard_manager
                .get_data_device(seat, &queue.handle(), seat.clone());
            data.set_device(Some(device));
        }

        let mut state = SessionState {
            common,
            offers: HashMap::new(),
            data_sources: HashMap::new(),
            owned_sources: Vec::new(),
        };

        queue
            .roundtrip(&mut state)
            .map_err(Error::WaylandCommunication)?;

        Ok(Self(Rc::new(RefCell::new(WlrSession { queue, state }))))
    }

    pub fn get_text(&self) -> Result<String, Error> {
        let mut session = self.0.borrow_mut();
        session.state.offers.clear();
        {
            let WlrSession { queue, state } = &mut *session;
            queue.roundtrip(state).map_err(Error::WaylandCommunication)?;
        }

        let offer = session
            .state
            .common
            .seats
            .values()
            .find_map(|seat| seat.offer.clone())
            .ok_or(Error::ClipboardEmpty)?;

        let mut mime_types = session
            .state
            .offers
            .remove(&offer)
            .unwrap_or_default();

        let mime_type = mime_types
            .iter()
            .position(|x| x == "text/plain;charset=utf-8")
            .or_else(|| mime_types.iter().position(|x| x == "UTF8_STRING"))
            .or_else(|| mime_types.iter().position(|x| is_text(x)))
            .map(|i| mime_types.swap_remove(i));

        let Some(mime_type) = mime_type else {
            return Err(Error::NoMimeType);
        };

        let (mut read, write) = pipe().map_err(Error::PipeCreation)?;
        offer.receive(mime_type.clone(), write.as_fd());
        mem::drop(write);

        {
            let WlrSession { queue, state } = &mut *session;
            queue.roundtrip(state).map_err(Error::WaylandCommunication)?;
        }

        let mut contents = Vec::new();
        read.read_to_end(&mut contents).map_err(Error::Io)?;
        Ok(String::from_utf8_lossy(&contents).to_string())
    }

    pub fn set_text(&self, value: &str) -> Result<(), Error> {
        let mut session = self.0.borrow_mut();
        for source in session.state.owned_sources.drain(..) {
            source.destroy();
        }

        let bytes: Arc<[u8]> = Arc::from(value.as_bytes());
        session.state.data_sources.clear();
        for mime in TEXT_MIMES {
            session
                .state
                .data_sources
                .insert((*mime).to_string(), bytes.clone());
        }

        {
            let WlrSession { queue, state } = &mut *session;
            let qh = queue.handle();
            let devices: Vec<_> = state
                .common
                .seats
                .values()
                .filter_map(|seat_data| seat_data.device.clone())
                .collect();
            for device in devices {
                let source = state.common.clipboard_manager.create_data_source(&qh);
                for mime in state.data_sources.keys() {
                    source.offer(mime.clone());
                }
                device.set_selection(Some(&source));
                state.owned_sources.push(source);
            }
            queue.roundtrip(state).map_err(Error::WaylandCommunication)?;
        }
        Ok(())
    }

    pub fn get_text_or_empty(&self) -> MyResult<String> {
        match self.get_text() {
            Ok(text) => Ok(text),
            Err(Error::ClipboardEmpty) | Err(Error::NoMimeType) | Err(Error::NoSeats) => {
                Ok(String::new())
            }
            Err(err) => Err(err.into()),
        }
    }

    pub fn set_text_result(&self, value: &str) -> MyResult<()> {
        self.set_text(value).map_err(Into::into)
    }
}

fn connect(
    primary: bool,
    socket_name: Option<OsString>,
) -> Result<(EventQueue<SessionState>, ConnectionState), Error> {
    let conn = match socket_name {
        Some(name) => {
            let mut socket_path = env::var_os("XDG_RUNTIME_DIR")
                .map(PathBuf::from)
                .ok_or(ConnectError::NoCompositor)
                .map_err(Error::WaylandConnection)?;
            if !socket_path.is_absolute() {
                return Err(Error::WaylandConnection(ConnectError::NoCompositor));
            }
            socket_path.push(name);
            let stream = UnixStream::connect(socket_path).map_err(Error::SocketOpenError)?;
            Connection::from_socket(stream)
        }
        None => Connection::connect_to_env(),
    }
    .map_err(Error::WaylandConnection)?;

    let (globals, queue) = registry_queue_init::<SessionState>(&conn).map_err(|err| match err {
        GlobalError::Backend(err) => Error::WaylandCommunication(err.into()),
        GlobalError::InvalidId(err) => panic!("missing wl_registry: {err:?}"),
    })?;
    let qh = queue.handle();

    let ext_manager = globals.bind(&qh, 1..=1, ()).ok().map(Manager::Ext);
    let wlr_v = if primary { 2 } else { 1 };
    let wlr_manager = || globals.bind(&qh, wlr_v..=wlr_v, ()).ok().map(Manager::Zwlr);

    let clipboard_manager = match ext_manager.or_else(wlr_manager) {
        Some(manager) => manager,
        None => {
            return Err(Error::MissingProtocol {
                name: "ext-data-control, or wlr-data-control",
                version: wlr_v,
            })
        }
    };

    let registry = globals.registry();
    let seats = globals.contents().with_list(|globals| {
        globals
            .iter()
            .filter(|global| global.interface == WlSeat::interface().name && global.version >= 2)
            .map(|global| {
                let seat = registry.bind(global.name, 2, &qh, ());
                (seat, SeatData::default())
            })
            .collect()
    });

    Ok((
        queue,
        ConnectionState {
            seats,
            clipboard_manager,
        },
    ))
}

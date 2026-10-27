//! Adapted from wl-clipboard-rs (MIT/Apache-2.0).

use std::os::fd::BorrowedFd;

use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Dispatch, QueueHandle};
use wayland_protocols::ext::data_control::v1::client::{
    ext_data_control_device_v1::ExtDataControlDeviceV1,
    ext_data_control_manager_v1::ExtDataControlManagerV1,
    ext_data_control_offer_v1::ExtDataControlOfferV1,
    ext_data_control_source_v1::ExtDataControlSourceV1,
};
use wayland_protocols_wlr::data_control::v1::client as zwlr;
use zwlr::zwlr_data_control_device_v1::ZwlrDataControlDeviceV1;
use zwlr::zwlr_data_control_manager_v1::ZwlrDataControlManagerV1;
use zwlr::zwlr_data_control_offer_v1::ZwlrDataControlOfferV1;
use zwlr::zwlr_data_control_source_v1::ZwlrDataControlSourceV1;

#[derive(Clone)]
pub enum Manager {
    Zwlr(ZwlrDataControlManagerV1),
    Ext(ExtDataControlManagerV1),
}

#[derive(Clone)]
pub enum Device {
    Zwlr(ZwlrDataControlDeviceV1),
    Ext(ExtDataControlDeviceV1),
}

#[derive(Clone)]
pub enum Source {
    Zwlr(ZwlrDataControlSourceV1),
    Ext(ExtDataControlSourceV1),
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Offer {
    Zwlr(ZwlrDataControlOfferV1),
    Ext(ExtDataControlOfferV1),
}

impl Manager {
    pub fn get_data_device<D, U>(&self, seat: &WlSeat, qh: &QueueHandle<D>, udata: U) -> Device
    where
        D: Dispatch<ZwlrDataControlDeviceV1, U> + 'static,
        D: Dispatch<ExtDataControlDeviceV1, U> + 'static,
        U: Send + Sync + 'static,
    {
        match self {
            Manager::Zwlr(manager) => Device::Zwlr(manager.get_data_device(seat, qh, udata)),
            Manager::Ext(manager) => Device::Ext(manager.get_data_device(seat, qh, udata)),
        }
    }

    pub fn create_data_source<D>(&self, qh: &QueueHandle<D>) -> Source
    where
        D: Dispatch<ZwlrDataControlSourceV1, ()> + 'static,
        D: Dispatch<ExtDataControlSourceV1, ()> + 'static,
    {
        match self {
            Manager::Zwlr(manager) => Source::Zwlr(manager.create_data_source(qh, ())),
            Manager::Ext(manager) => Source::Ext(manager.create_data_source(qh, ())),
        }
    }
}

impl Device {
    pub fn destroy(&self) {
        match self {
            Device::Zwlr(device) => device.destroy(),
            Device::Ext(device) => device.destroy(),
        }
    }

    pub fn set_selection(&self, source: Option<&Source>) {
        match self {
            Device::Zwlr(device) => device.set_selection(source.map(Source::zwlr)),
            Device::Ext(device) => device.set_selection(source.map(Source::ext)),
        }
    }
}

impl Source {
    pub fn destroy(&self) {
        match self {
            Source::Zwlr(source) => source.destroy(),
            Source::Ext(source) => source.destroy(),
        }
    }

    pub fn offer(&self, mime_type: String) {
        match self {
            Source::Zwlr(source) => source.offer(mime_type),
            Source::Ext(source) => source.offer(mime_type),
        }
    }

    fn zwlr(&self) -> &ZwlrDataControlSourceV1 {
        match self {
            Source::Zwlr(source) => source,
            Source::Ext(_) => panic!("non-Zwlr source"),
        }
    }

    fn ext(&self) -> &ExtDataControlSourceV1 {
        match self {
            Source::Ext(source) => source,
            Source::Zwlr(_) => panic!("non-Ext source"),
        }
    }
}

impl Offer {
    pub fn destroy(&self) {
        match self {
            Offer::Zwlr(offer) => offer.destroy(),
            Offer::Ext(offer) => offer.destroy(),
        }
    }

    pub fn receive(&self, mime_type: String, fd: BorrowedFd<'_>) {
        match self {
            Offer::Zwlr(offer) => offer.receive(mime_type, fd),
            Offer::Ext(offer) => offer.receive(mime_type, fd),
        }
    }
}

impl From<ZwlrDataControlSourceV1> for Source {
    fn from(v: ZwlrDataControlSourceV1) -> Self {
        Self::Zwlr(v)
    }
}

impl From<ExtDataControlSourceV1> for Source {
    fn from(v: ExtDataControlSourceV1) -> Self {
        Self::Ext(v)
    }
}

impl From<ZwlrDataControlOfferV1> for Offer {
    fn from(v: ZwlrDataControlOfferV1) -> Self {
        Self::Zwlr(v)
    }
}

impl From<ExtDataControlOfferV1> for Offer {
    fn from(v: ExtDataControlOfferV1) -> Self {
        Self::Ext(v)
    }
}

macro_rules! impl_dispatch_manager {
    ($handler:ty => [$($iface:ty),*]) => {
        $(
            impl Dispatch<$iface, ()> for $handler {
                fn event(
                    _state: &mut Self,
                    _proxy: &$iface,
                    _event: <$iface as wayland_client::Proxy>::Event,
                    _data: &(),
                    _conn: &wayland_client::Connection,
                    _qhandle: &wayland_client::QueueHandle<Self>,
                ) {
                }
            }
        )*
    };

    ($handler:ty) => {
        impl_dispatch_manager!($handler => [
            ZwlrDataControlManagerV1,
            ExtDataControlManagerV1
        ]);
    };
}
pub(crate) use impl_dispatch_manager;

macro_rules! impl_dispatch_device {
    ($handler:ty, $udata:ty, $code:expr => [$(($iface:ty, $opcode:path, $offer:ty)),*]) => {
        $(
            impl Dispatch<$iface, $udata> for $handler {
                fn event(
                    state: &mut Self,
                    _proxy: &$iface,
                    event: <$iface as wayland_client::Proxy>::Event,
                    data: &$udata,
                    _conn: &wayland_client::Connection,
                    _qhandle: &wayland_client::QueueHandle<Self>,
                ) {
                    type Event = <$iface as wayland_client::Proxy>::Event;
                    ($code)(state, event, data)
                }

                event_created_child!($handler, $iface, [
                    $opcode => ($offer, ()),
                ]);
            }
        )*
    };

    ($handler:ty, $udata:ty, $code:expr) => {
        impl_dispatch_device!($handler, $udata, $code => [
            (
                ZwlrDataControlDeviceV1,
                wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_device_v1::EVT_DATA_OFFER_OPCODE,
                ZwlrDataControlOfferV1
            ),
            (
                ExtDataControlDeviceV1,
                wayland_protocols::ext::data_control::v1::client::ext_data_control_device_v1::EVT_DATA_OFFER_OPCODE,
                ExtDataControlOfferV1
            )
        ]);
    };
}
pub(crate) use impl_dispatch_device;

macro_rules! impl_dispatch_source {
    ($handler:ty, $code:expr => [$($iface:ty),*]) => {
        $(
            impl Dispatch<$iface, ()> for $handler {
                fn event(
                    state: &mut Self,
                    proxy: &$iface,
                    event: <$iface as wayland_client::Proxy>::Event,
                    _data: &(),
                    _conn: &wayland_client::Connection,
                    _qhandle: &wayland_client::QueueHandle<Self>,
                ) {
                    type Event = <$iface as wayland_client::Proxy>::Event;
                    let source = crate::wlr_backend::data_control::Source::from(proxy.clone());
                    ($code)(state, source, event)
                }
            }
        )*
    };

    ($handler:ty, $code:expr) => {
        impl_dispatch_source!($handler, $code => [
            ZwlrDataControlSourceV1,
            ExtDataControlSourceV1
        ]);
    };
}
pub(crate) use impl_dispatch_source;

macro_rules! impl_dispatch_offer {
    ($handler:ty, $code:expr => [$($iface:ty),*]) => {
        $(
            impl Dispatch<$iface, ()> for $handler {
                fn event(
                    state: &mut Self,
                    proxy: &$iface,
                    event: <$iface as wayland_client::Proxy>::Event,
                    _data: &(),
                    _conn: &wayland_client::Connection,
                    _qhandle: &wayland_client::QueueHandle<Self>,
                ) {
                    type Event = <$iface as wayland_client::Proxy>::Event;
                    let offer = crate::wlr_backend::data_control::Offer::from(proxy.clone());
                    ($code)(state, offer, event)
                }
            }
        )*
    };

    ($handler:ty, $code:expr) => {
        impl_dispatch_offer!($handler, $code => [
            ZwlrDataControlOfferV1,
            ExtDataControlOfferV1
        ]);
    };
}
pub(crate) use impl_dispatch_offer;

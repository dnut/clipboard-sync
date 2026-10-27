use crate::wlr_backend::data_control::{Device, Offer};

#[derive(Default)]
pub struct SeatData {
    pub name: Option<String>,
    pub device: Option<Device>,
    pub offer: Option<Offer>,
    pub primary_offer: Option<Offer>,
}

impl SeatData {
    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    pub fn set_device(&mut self, device: Option<Device>) {
        let old_device = self.device.take();
        self.device = device;
        if let Some(device) = old_device {
            device.destroy();
        }
    }

    pub fn set_offer(&mut self, new_offer: Option<Offer>) {
        let old_offer = self.offer.take();
        self.offer = new_offer;
        if let Some(offer) = old_offer {
            offer.destroy();
        }
    }
}

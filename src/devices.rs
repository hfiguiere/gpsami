// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::sync::Arc;

use gettextrs::gettext as i18n;
use serde::Deserialize;

use crate::drivers;
use crate::gpsbabel;

/// Device static capability
#[derive(Clone, Debug, Deserialize)]
pub struct Capability {
    pub can_erase: bool,
    pub can_erase_only: bool,
    can_log_enable: bool,
    can_shutoff: bool,
}

/// Describe a device
#[derive(Clone, Debug, Deserialize)]
pub struct Desc {
    pub id: String,
    pub label: String,
    cap: Capability,
    driver: String,
}

/// The device database.
#[derive(Clone, Debug, Deserialize)]
struct DeviceDb {
    devices: Vec<Desc>,
    drivers: Vec<drivers::Desc>,
}

/// The device manager. Where the magic happens.
pub struct Manager {
    model: Option<String>,
    port: Option<String>,
    devices: Vec<Desc>,
    drivers: Vec<drivers::Desc>,

    udev_context: libudev::Context,
    pub gudev_client: gudev::Client, // gudev client. We need to keep it alive.
    device_filter: Option<Vec<drivers::PortType>>,
}

impl Manager {
    pub fn new() -> Self {
        let devices_db: DeviceDb = serde_json::from_str(include_str!("devices.json")).unwrap();

        let client = gudev::Client::new(&["tty"]);

        let context = libudev::Context::new();
        if context.is_err() {
            // XXX not sure how do handle the error.
        }

        Manager {
            model: None,
            port: None,
            devices: devices_db.devices,
            drivers: devices_db.drivers,
            udev_context: context.unwrap(),
            gudev_client: client,
            device_filter: None,
        }
    }

    fn listen_for_devices(&mut self, port_type: Vec<drivers::PortType>) {
        // XXX set the listener event filtering...
        self.device_filter = Some(port_type);
    }

    pub fn set_model(&mut self, model: &str) {
        let port_filter = self.get_port_filter_for_model(model);
        self.model = Some(model.to_owned());
        self.listen_for_devices(port_filter);
    }

    pub fn set_port(&mut self, port: &str) {
        self.port = Some(port.to_owned());
    }

    pub fn devices_desc(&self) -> &Vec<Desc> {
        &self.devices
    }

    pub fn device_capability(&self, model: &str) -> Option<Capability> {
        if model.is_empty() {
            return None;
        }
        // XXX this is suboptimal.
        self.devices
            .iter()
            .find(|&device| device.id == *model)
            .map(|device| device.cap.clone())
    }

    fn list_ports(&self, port_filters: Vec<drivers::PortType>) -> Vec<drivers::Port> {
        let mut dv: Vec<drivers::Port> = vec![];
        for port_filter in port_filters {
            let enumerator = libudev::Enumerator::new(&self.udev_context);
            if enumerator.is_err() {
                return Vec::new();
            }

            let mut e = enumerator.unwrap();
            match port_filter {
                drivers::PortType::UsbSerial => {
                    if e.match_subsystem("tty").is_err() {
                        println!("match_subsystem(\"tty\") failed");
                        return Vec::new();
                    }
                    if e.match_property("ID_BUS", "usb").is_err() {
                        println!("match_property(\"bus = usb\") failed");
                        return Vec::new();
                    }
                }
                drivers::PortType::RfComm => {
                    // it seems the only way is to use this filter.
                    if e.match_subsystem("tty").is_err() {
                        println!("match_subsystem(\"tty\") failed");
                        return Vec::new();
                    }
                    if e.match_sysname("rfcomm[0-9]").is_err() {
                        println!("match_sysname(\"rfcomm\") failed");
                        return Vec::new();
                    }
                }
                _ => {}
            }

            let devices = e.scan_devices();
            if devices.is_err() {
                return Vec::new();
            }
            let ds = devices.unwrap();
            let mut dv2: Vec<drivers::Port> = ds
                .map(|dev| {
                    let path = dev.devnode().unwrap().to_path_buf();
                    let id = dev.sysname().to_string_lossy().into_owned();
                    let label = match dev.property_value("ID_MODEL_FROM_DATABASE") {
                        Some(s) => s.to_string_lossy().into_owned(),
                        None => i18n("(Unknown)"),
                    };
                    drivers::Port { id, label, path }
                })
                .collect();
            dv.append(&mut dv2);
        }

        dv
    }

    fn get_port_filter_for_model(&self, model: &str) -> Vec<drivers::PortType> {
        match self.devices.iter().find(|&device| device.id == model) {
            Some(device) => match self
                .drivers
                .iter()
                .find(|&driver| driver.id == device.driver)
            {
                Some(driver) => driver.ports.clone(),
                _ => vec![drivers::PortType::None],
            },
            None => vec![drivers::PortType::None],
        }
    }

    pub fn get_ports_for_model(&self, model: &str) -> Option<Vec<drivers::Port>> {
        let port_filter = self.get_port_filter_for_model(model);
        Some(self.list_ports(port_filter))
    }

    // Get a driver for the device from the current manager.
    pub fn get_device(&self) -> Option<Arc<dyn drivers::Driver + Send + Sync>> {
        self.model.as_ref()?;

        let capability: Capability;
        let driver_id = match self.devices.iter().find(|&device| {
            if let Some(ref model) = self.model {
                return &device.id == model;
            }
            false
        }) {
            Some(device) => {
                capability = device.cap.clone();
                device.driver.clone()
            }
            None => return None,
        };
        match driver_id.as_str() {
            "baroiq" | "dg-100" | "dg-200" | "navilink" | "m241" | "mtk" => match self.port {
                Some(ref p) => Some(Arc::new(gpsbabel::GpsBabel::new(driver_id, p, capability))),
                _ => None,
            },
            _ => None,
        }
    }
}

impl Capability {
    //    pub fn new() -> Self {
    //        Capability {
    //            can_erase: false,
    //            can_erase_only: false,
    //            can_log_enable: false,
    //            can_shutoff: false,
    //        }
    //    }
}

#[cfg(test)]
#[test]
fn test_database() {
    // This test that the database has a valid syntax....
    let devices_db: DeviceDb = serde_json::from_str(include_str!("devices.json")).unwrap();
    assert!(!devices_db.devices.is_empty());
}

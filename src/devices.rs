use libudev;
use gudev;
use rustc_serialize::json;

use drivers;
use gpsbabel;

/// Device static capability
#[derive(Clone, Debug, RustcDecodable)]
pub struct Capability {
    pub can_erase: bool,
    pub can_erase_only: bool,
    can_log_enable: bool,
    can_shutoff: bool,
}

/// Describe a device
#[derive(Clone, Debug, RustcDecodable)]
pub struct Desc {
    pub id: String,
    pub label: String,
    cap: Capability,
    driver: String,
}

/// The device database.
#[derive(Clone, Debug, RustcDecodable)]
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
    device_filter: Option<drivers::PortType>,
}

impl Manager {
    pub fn new() -> Self {
        let devices_db: DeviceDb = json::decode(
            include_str!("devices.json")
            ).unwrap();

        let client = gudev::Client::new(&vec!["tty"]);

        let context = libudev::Context::new();
        if context.is_err() {
            // XXX not sure how do handle the error.
        }

        let manager = Manager {
            model: None, port: None,
            devices: devices_db.devices,
            drivers: devices_db.drivers,
            udev_context: context.unwrap(),
            gudev_client: client,
            device_filter: None
        };
        manager
    }

    fn listen_for_devices(&mut self, port_type: drivers::PortType) {
        // XXX set the listener event filtering...
        self.device_filter = Some(port_type);
    }

    pub fn set_model(&mut self, model: String) {
        let port_filter = self.get_port_filter_for_model(&model);
        self.model = Some(model);
        self.listen_for_devices(port_filter);
    }

    pub fn set_port(&mut self, port: String) {
        self.port = Some(port);
    }

    pub fn devices_desc(&self) -> &Vec<Desc> {
        &self.devices
    }

    pub fn device_capability(&self, model: &String) -> Option<Capability> {
        if model.is_empty() {
            return None;
        }
        // XXX this is suboptimal.
        match self.devices.iter().find(|&device| device.id == *model) {
            Some(device) =>
                Some(device.cap.clone()),
            None =>
                None,
        }
    }

    fn list_ports(&self, port_filter: drivers::PortType) -> Vec<drivers::Port> {
        let enumerator = libudev::Enumerator::new(&self.udev_context);
        if enumerator.is_err() {
            return Vec::new();
        }

        let mut e = enumerator.unwrap();
        match port_filter {
            drivers::PortType::UsbSerial => {
                e.match_subsystem("tty");
                e.match_property("ID_BUS", "usb");
            },
            _ => {
            },
        }

        let devices = e.scan_devices();
        if devices.is_err() {
            return Vec::new();
        }
        let ds = devices.unwrap();
        let dv: Vec<drivers::Port> = ds.map(
            |dev|  {
                let path = dev.devnode().unwrap().to_path_buf();
                let id = dev.sysname().to_string_lossy().into_owned();
                let label = dev.property_value("ID_MODEL_FROM_DATABASE").unwrap().to_string_lossy().into_owned();
                drivers::Port { id: id, label: label, path: path }
            }).collect();
        return dv;
    }

    fn get_port_filter_for_model(&self, model: &str) -> drivers::PortType {
        let port_filter = match self.devices.iter().find(
            |&device| {
                &device.id == model
            }) {
            Some(device) => {
                match self.drivers.iter().find(
                    |&driver| {
                        driver.id == device.driver
                    }) {
                    Some(driver) => driver.ports.clone(),
                    _=> drivers::PortType::None,
                }
            },
            None =>
                drivers::PortType::None
        };
        return port_filter
    }

    pub fn get_ports_for_model(&self, model: &str)
                               -> Option<Vec<drivers::Port>> {
        let port_filter = self.get_port_filter_for_model(model);
        Some(self.list_ports(port_filter))
    }

    // Get a driver for the device from the current manager.
    pub fn get_device(&self) -> Option<Box<drivers::Driver>> {
        if self.model == None {
            return None;
        }
        let capability: Capability;
        let driver_id = match self.devices.iter().find(
            |&device| {
                if let Some(ref model) = self.model {
                    return &device.id == model;
                }
                false
            }) {
            Some(device) => {
                capability = device.cap.clone();
                device.driver.clone()
            },
            None =>
                return None
        };
        match driver_id.as_str() {
            "m241" |
            "mtk" => {
                match self.port {
                    Some(ref p) =>
                        Some(Box::new(gpsbabel::GpsBabel::new(driver_id,
                                                              p, capability))),
                    _ => None
                }
            },
            _ =>
                None
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
    let devices_db: DeviceDb = json::decode(
        include_str!("devices.json")
            ).unwrap();
    assert!(!devices_db.devices.is_empty());
}

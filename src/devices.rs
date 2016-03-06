


#[derive(Clone, Debug)]
pub struct Port {
    pub id: String,
    pub label: String,
}

/// Device static capability
#[derive(Clone, Debug)]
pub struct Capability {
    pub can_erase: bool,
    can_erase_only: bool,
    can_log_enable: bool,
    can_shutoff: bool,
}

#[derive(Clone, Debug)]
pub struct Desc {
    pub id: &'static str,
    pub label: &'static str,
    cap: Capability,
}

static DEVICES : [Desc; 1] = [
    Desc {
        id: "holux",
        label: "Holux",
        cap: Capability {
            can_erase: true,
            can_erase_only: true,
            can_log_enable: false,
            can_shutoff: false
        }
    }
    ];

pub struct Manager {
    model: Option<String>,
    port: Option<String>,
    devices: Vec<Desc>,
}

impl Manager {

    pub fn new() -> Self {
        Manager { model: None, port: None, devices: DEVICES.to_vec() }
    }

    pub fn set_model(&mut self, model: String) {
        self.model = Some(model);
    }

    pub fn set_port(&mut self, port: String) {
        self.port = Some(port);
    }

    pub fn devices_desc(&self) -> &Vec<Desc> {
        &self.devices
    }

    pub fn device_capability(&self, model: &String) -> Capability {
        if model.is_empty() {
            return Capability::new();
        }
        // XXX this is suboptimal.
        for device in &self.devices {
            if device.id == model {
                return device.cap.clone();
            }
        }
        Capability::new()
    }

    pub fn get_ports_for_model(&self, _ /*model*/: &String) -> Vec<Port> {
        return vec![ Port { id: "foo".to_string(), label: "bar".to_string() } ];
    }
}

impl Capability {

    pub fn new() -> Self {
        Capability {
            can_erase: false,
            can_erase_only: false,
            can_log_enable: false,
            can_shutoff: false,
        }
    }
}

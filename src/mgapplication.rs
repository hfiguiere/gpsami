use gio;
use glib;
use gtk;
use gtk::prelude::*;

use std;
use std::path;
use std::rc::Rc;
use std::cell::RefCell;

use devices;
use drivers;
use utils;
use ::Format;

pub struct MgApplication {
    win: gtk::ApplicationWindow,
    erase_checkbtn: gtk::CheckButton,
    model_combo: gtk::ComboBox,
    port_entry: gtk::Entry,

    device_manager: devices::Manager,
    prefs_store: glib::KeyFile,

    model_changed_signal: u64,
    port_changed_signal: u64,

    output_dest_dir: path::PathBuf,
}

impl MgApplication {

    pub fn new(gapp: &gtk::Application) -> Rc<RefCell<MgApplication>> {

        let builder = gtk::Builder::new_from_string(include_str!("mgwindow.ui"));
        let window: gtk::ApplicationWindow = builder.get_object("main_window").unwrap();
        let erase_checkbtn: gtk::CheckButton = builder.get_object("erase_checkbtn").unwrap();
        let model_combo: gtk::ComboBox = builder.get_object("model_combo").unwrap();
        let port_entry: gtk::Entry = builder.get_object("port_entry").unwrap();
        let output_dir_chooser: gtk::FileChooserButton = builder.get_object("output_dir_chooser").unwrap();

        gapp.add_window(&window);

        let app = MgApplication {
            win: window,
            erase_checkbtn: erase_checkbtn,
            model_combo: model_combo,
            port_entry: port_entry,
            device_manager: devices::Manager::new(),
            prefs_store: glib::KeyFile::new(),
            model_changed_signal: 0,
            port_changed_signal: 0,
            output_dest_dir: path::PathBuf::new()
        };

        let me = Rc::new(RefCell::new(app));
        {
            let me_too = me.clone();
            let signal_id = me.borrow_mut().model_combo.connect_changed(move |combo| {
                if let Some(id) = combo.get_active_id() {
                    me_too.borrow_mut().model_changed(&id);
                }
            });
            me.borrow_mut().model_changed_signal = signal_id;
        }
        {
            let me_too = me.clone();
            let signal_id = me.borrow_mut().port_entry.connect_changed(move |entry| {
                if let Some(id) = entry.get_text() {
                    me_too.borrow_mut().port_changed(&id);
                }
            });
            me.borrow_mut().port_changed_signal = signal_id;
        }
        {
            let me_too = me.clone();
            let dload_action = gio::SimpleAction::new("download", None);
            dload_action.connect_activate(move |_,_| {
                me_too.borrow().do_download();
            });
            dload_action.set_enabled(false);
            me.borrow_mut().win.add_action(&dload_action);
        }

        {
            let me_too = me.clone();
            let erase_action = gio::SimpleAction::new("erase", None);
            erase_action.connect_activate(move |_,_| {
                me_too.borrow().do_erase();
            });
            erase_action.set_enabled(false);
            me.borrow_mut().win.add_action(&erase_action);
        }
        {
            let me_too = me.clone();
            output_dir_chooser.connect_file_set(move |w| {
                let file_name = w.get_filename();
                match file_name {
                    Some(f) => {
                        me_too.borrow_mut().set_output_destination_dir(f.as_ref());
                        me_too.borrow().prefs_store.set_string("output", "dir",
                                                               f.to_str().unwrap());
                        if me_too.borrow().save_settings().is_err() {
                            println!("Error loading settings");
                        }
                    },
                    _ => {}
                }
            });
        }

        if me.borrow_mut().load_settings().is_err() {
            println!("Error loading settings");
        }
        output_dir_chooser.set_current_folder(
            me.borrow().prefs_store.get_string("output", "dir").unwrap_or("".to_owned()));
        me
    }

    fn do_download(&self) {
        let device = self.device_manager.get_device();
        if device.is_none() {
            println!("nodriver");
        } else {
            let output_file: path::PathBuf;
            let chooser = gtk::FileChooserDialog::new(Some("Save File"),
                                                      Some(&self.win),
                                                      gtk::FileChooserAction::Save);
            chooser.add_buttons(&[
                ("Save", gtk::ResponseType::Ok.into()),
                ("Cancel", gtk::ResponseType::Cancel.into()),
                ]);
            chooser.set_current_folder(self.prefs_store
                                       .get_string("output", "dir")
                                       .unwrap_or("".to_owned()));
            if chooser.run() == gtk::ResponseType::Ok.into() {
                let result = chooser.get_filename();
                chooser.destroy();
                match result {
                    Some(f) => output_file = f,
                    _ => return,
                }
            } else {
                chooser.destroy();
                return;
            }
            let mut d = device.unwrap();
            if d.open() {
                match d.download(Format::Gpx, false) {
                    Ok(temp_output_filename) => {
                        println!("success {}", temp_output_filename.to_str().unwrap());
                        match std::fs::copy(temp_output_filename, &output_file) {
                            Err(e) =>
                                self.report_error(&format!("Failed to save {}: {}",
                                                           output_file.to_str().unwrap(), e)),
                            _ => {}
                        }
                    },
                    Err(e) =>
                        self.report_error(&format!("Failed to download GPS data: {}", e)),
                }
            }
        }
    }

    fn report_error(&self, message: &str) {
        let dialog = gtk::MessageDialog::new(Some(&self.win), gtk::DIALOG_MODAL,
                                             gtk::MessageType::Error,
                                             gtk::ButtonsType::Close,
                                             message);
        dialog.run();
        dialog.destroy();
    }

    fn do_erase(&self) {
        let device = self.device_manager.get_device();
        if device.is_none() {
            println!("nodriver");
        } else {
            let mut d = device.unwrap();
            if d.open() {
                match d.erase() {
                    drivers::Error::None =>
                        println!("success erasing"),
                    _ =>
                        println!("failed erasing"),
                }
            }
        }
    }

    fn settings_dir() -> path::PathBuf {
        // XXX replace this by glib stuff when we can.
        // Also we treat a failure of this as fatal.
        let mut path: path::PathBuf = std::env::home_dir().unwrap();
        path.push(".magellan");
        path
    }

    fn save_settings(&self) -> Result<(), glib::Error> {
        let mut path = Self::settings_dir();
        path.push("magellan.ini");
        self.prefs_store.save_to_file(path.to_str().unwrap())
    }

    fn set_output_destination_dir(&mut self, output: &path::Path) {
        self.output_dest_dir = output.to_owned();
    }

    pub fn load_settings(&mut self) -> Result<(), glib::Error> {
        let mut path = Self::settings_dir();
        match std::fs::create_dir_all(path.clone()) {
            Err(e) =>
                return Err(
                    glib::Error::new(glib::FileError::Failed,
                                     &format!("Can't create settings dir '{:?}': {}", path, e))),
            Ok(_) => {
            },
        }
        path.push("magellan.ini");

        match self.prefs_store.load_from_file(path, glib::KEY_FILE_NONE) {
            Err(e) => {
                println!("error with g_key_file {}", e);
                Err(e)
            },
            Ok(_) => {
                Ok(())
            },
        }
    }

    /// Start the app.
    pub fn start(&mut self) {

        self.populate_model_combo();
        self.win.show_all();

        // XXX used the stored value here.
    }

    fn populate_port_combo(&mut self, /*ports*/ _: &Vec<drivers::Port>) {
// XXX fix
//        self.port_combo.remove_all();
//        for port in ports {
//            println!("adding port {:?}", port);
//            self.port_combo.append_text(&port.id);
//        }
    }

    fn populate_model_combo(&mut self) {
        let store = gtk::ListStore::new(&[
            gtk::Type::String, gtk::Type::String
                ]);

        utils::setup_text_combo(&self.model_combo, &store);

        {
            let devices = self.device_manager.devices_desc();
            for dev in devices {
                utils::add_text_row(&store, &dev.id, &dev.label);
            }
        }

        let model = self.prefs_store.get_string("device", "model").unwrap_or("".to_string());
        let port = self.prefs_store.get_string("device", "port").unwrap_or("".to_string());

        // XXX this is a hack to not have the signal called as we'll end up
        // recursively borrow_mut self via the RefCell
        glib::signal_handler_block(&self.model_combo, self.model_changed_signal);
        self.model_combo.set_active_id(Some(&model));
        self.model_changed(&model);
        glib::signal_handler_unblock(&self.model_combo, self.model_changed_signal);
        glib::signal_handler_block(&self.port_entry, self.port_changed_signal);
        self.port_entry.set_text(&port);
        self.port_changed(&port);
        glib::signal_handler_unblock(&self.port_entry, self.port_changed_signal);
    }

    fn model_changed(&mut self, id: &String) {
        println!("model changed to {}", id);
        self.prefs_store.set_string("device", "model", &id);
        if self.save_settings().is_err() {
            println!("Error loading settings");
        }

        let cap = self.device_manager.device_capability(id);
        if cap.is_some() {
            self.update_device_capability(&cap.unwrap());
            self.device_manager.set_model(id.clone());
            let ports = self.device_manager.get_ports_for_model(id);
            self.populate_port_combo(&ports);
        } else {
            // XXX clear device.
        }
    }

    fn update_device_capability(&self, capability: &devices::Capability) {
        self.erase_checkbtn.set_sensitive(capability.can_erase);
        match self.win.lookup_action("erase") {
            Some(a) => match a.downcast::<gio::SimpleAction>() {
                Ok(sa) =>
                    sa.set_enabled(capability.can_erase_only),
                _ => {},
            },
            _ => {},
        }
    }

    fn port_changed(&mut self, id: &str) {
        self.prefs_store.set_string("device", "port", id);
        if self.save_settings().is_err() {
            println!("Error loading settings");
        }

        self.device_manager.set_port(id.to_string());
        match self.win.lookup_action("download") {
            Some(a) => match a.downcast::<gio::SimpleAction>() {
                Ok(sa) =>
                    sa.set_enabled(id != ""),
                _ => {},
            },
            _ => {},
        }
    }
}

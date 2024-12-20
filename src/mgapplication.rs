//
// Copyright (C) 2016-2024 Hubert Figuière
//
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

use async_channel::Sender;
use gettextrs::gettext as i18n;
use gtk4 as gtk;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gudev::prelude::DeviceExt;
use gudev::prelude::*;

use std::cell::RefCell;
use std::path;
use std::rc::Rc;
use std::sync::Arc;
use std::thread;

use crate::devices;
use crate::drivers;
use crate::file_chooser_button::FileChooserButton;
use crate::utils;
use crate::Format;

enum UiState {
    Idle,
    InProgress,
}

pub enum MgAction {
    RescanDevices,
    ModelChanged(String),
    PortChanged(String),
    StartErase,
    DoneErase(drivers::Result<()>),
    StartDownload,
    DoneDownload(drivers::Result<()>),
    SetOutputDir(path::PathBuf),
}

fn post_event(sender: &Sender<MgAction>, action: MgAction) {
    let sender = sender.clone();
    glib::MainContext::default().spawn(async move {
        if let Err(err) = sender.send(action).await {
            log::error!("Sender error: {err}");
        }
    });
}

pub struct MgApplication {
    gapp: adw::Application,
    window_id: u32,
    content_box: gtk::Box,
    erase_checkbtn: gtk::CheckButton,
    model_combo: gtk::ComboBox,
    model_store: gtk::ListStore,
    port_combo: gtk::ComboBox,
    port_store: gtk::ListStore,
    toast_overlay: adw::ToastOverlay,

    device_manager: devices::Manager,
    prefs_store: glib::KeyFile,

    output_dest_dir: path::PathBuf,
    sender: Sender<MgAction>,
}

impl MgApplication {
    pub fn new(gapp: &adw::Application) -> Rc<RefCell<Self>> {
        let builder = gtk::Builder::from_resource("/net/figuiere/gpsami/mgwindow.ui");
        let content_box = builder.object::<gtk::Box>("content_box").unwrap();
        let window = gtk::ApplicationWindow::builder()
            .application(gapp)
            .default_height(400)
            .default_width(400)
            .child(&content_box)
            .build();
        let erase_checkbtn: gtk::CheckButton = builder.object("erase_checkbtn").unwrap();
        let model_combo: gtk::ComboBox = builder.object("model_combo").unwrap();
        let port_combo: gtk::ComboBox = builder.object("port_combo").unwrap();
        let output_dir_chooser: FileChooserButton = builder.object("output_dir_chooser").unwrap();
        let toast_overlay = builder
            .object::<adw::ToastOverlay>("toast_overlay")
            .unwrap();

        let (sender, receiver) = async_channel::unbounded::<MgAction>();

        let sender2 = sender.clone();
        model_combo.connect_changed(move |combo| {
            if let Some(id) = combo.active_id() {
                post_event(&sender2, MgAction::ModelChanged(id.to_string()));
            }
        });
        let sender2 = sender.clone();
        port_combo.connect_changed(move |entry| {
            if let Some(id) = entry.active_id() {
                post_event(&sender2, MgAction::PortChanged(id.to_string()));
            }
        });
        let dload_action = gio::SimpleAction::new("download", None);
        let sender2 = sender.clone();
        dload_action.connect_activate(move |_, _| {
            post_event(&sender2, MgAction::StartDownload);
        });
        dload_action.set_enabled(false);
        window.add_action(&dload_action);

        let erase_action = gio::SimpleAction::new("erase", None);
        let sender2 = sender.clone();
        erase_action.connect_activate(move |_, _| {
            post_event(&sender2, MgAction::StartErase);
        });
        erase_action.set_enabled(false);
        window.add_action(&erase_action);

        output_dir_chooser.connect_local(
            "file-set",
            true,
            glib::clone!(
                #[weak]
                output_dir_chooser,
                #[strong]
                sender,
                #[upgrade_or]
                None,
                move |_| {
                    let file_name = output_dir_chooser.filename();
                    if let Some(f) = file_name {
                        post_event(&sender, MgAction::SetOutputDir(f));
                    }
                    None
                }
            ),
        );

        let device_manager = devices::Manager::new();
        let sender2 = sender.clone();
        device_manager
            .gudev_client
            .connect_uevent(move |_, action, device| {
                if let Some(subsystem) = device.subsystem() {
                    log::debug!("received event {action} {subsystem}");
                }
                post_event(&sender2, MgAction::RescanDevices);
            });

        let app = MgApplication {
            gapp: gapp.clone(),
            window_id: window.id(),
            content_box,
            erase_checkbtn,
            model_combo,
            model_store: gtk::ListStore::new(&[glib::Type::STRING, glib::Type::STRING]),
            port_combo,
            port_store: gtk::ListStore::new(&[glib::Type::STRING, glib::Type::STRING]),
            toast_overlay,

            device_manager,
            prefs_store: glib::KeyFile::new(),
            output_dest_dir: path::PathBuf::new(),
            sender,
        };

        let me = Rc::new(RefCell::new(app));

        let metoo = me.clone();

        glib::spawn_future_local(glib::clone!(
            #[strong(rename_to = rx)]
            receiver,
            #[strong]
            metoo,
            async move {
                log::debug!("attaching for {}", std::any::type_name::<MgAction>());
                while let Ok(message) = rx.recv().await {
                    metoo.borrow_mut().process_event(message);
                }
                log::debug!("terminating {}", std::any::type_name::<MgAction>());
            }
        ));

        if me.borrow_mut().load_settings().is_err() {
            log::error!("Error loading settings");
        }

        if let Ok(output_dir) = me.borrow().prefs_store.string("output", "dir") {
            output_dir_chooser.set_filename(path::PathBuf::from(output_dir.as_str()));
        }
        me
    }

    fn do_download(&mut self) {
        let device = self.device_manager.get_device();
        if device.is_none() {
            log::debug!("nodriver");
            post_event(
                &self.sender,
                MgAction::DoneDownload(Err(drivers::Error::NoDriver)),
            );
            return;
        }
        let device = device.unwrap();

        let window = self.gapp.window_by_id(self.window_id);
        let chooser = gtk::FileChooserDialog::new(
            Some("Save File"),
            window.as_ref(),
            gtk::FileChooserAction::Save,
            &[],
        );
        chooser.add_buttons(&[
            ("Save", gtk::ResponseType::Ok),
            ("Cancel", gtk::ResponseType::Cancel),
        ]);
        if let Ok(output_dir) = self.prefs_store.string("output", "dir") {
            let _ = chooser.set_current_folder(Some(&gio::File::for_path(output_dir.as_str())));
        }
        chooser.show();

        let erase = self.erase_checkbtn.is_active();
        chooser.connect_response(glib::clone!(
            #[strong(rename_to = sender)]
            self.sender,
            #[strong]
            device,
            move |chooser, r| {
                log::debug!("Response {r}");

                chooser.close();
                match r {
                    gtk::ResponseType::Ok => {
                        if let Some(output_file) = chooser.file().and_then(|f| f.path()) {
                            Self::really_do_download(
                                sender.clone(),
                                device.clone(),
                                erase,
                                output_file,
                            );
                        }
                    }
                    gtk::ResponseType::DeleteEvent => {}
                    _ => post_event(
                        &sender,
                        MgAction::DoneDownload(Err(drivers::Error::Cancelled)),
                    ),
                }
                chooser.close();
            }
        ));
    }

    fn really_do_download(
        sender: Sender<MgAction>,
        device: Arc<dyn drivers::Driver + Send + Sync>,
        erase: bool,
        output_file: path::PathBuf,
    ) {
        print_on_err!(
            thread::Builder::new().name("downloader".into()).spawn(move || {
                let result = if device.open() {
                    let tempdir = tempfile::tempdir();
                    if let Ok(tempdir) = tempdir {
                        MgAction::DoneDownload(
                            device.download(Format::Gpx, erase, &tempdir)
                                .and_then(|temp_output_filename| {
                                    log::debug!(
                                        "success {temp_output_filename:?} -> will copy to {output_file:?}"
                                            );
                                    std::fs::copy(temp_output_filename, output_file).map(|_| ()).map_err(drivers::Error::from)
                                })
                            )
                    } else {
                        MgAction::DoneDownload(
                            tempdir.map(|_| ())
                                .map_err(drivers::Error::from)
                        )
                    }
                } else {
                    MgAction::DoneDownload(Err(drivers::Error::Failed(i18n("Open failed."))))
                };
                post_event(&sender, result);
            }));
    }

    fn report_error(&self, message: &str, reason: &str) {
        let window = self.gapp.window_by_id(self.window_id);
        let dialog = gtk::MessageDialog::new(
            window.as_ref(),
            gtk::DialogFlags::MODAL,
            gtk::MessageType::Error,
            gtk::ButtonsType::Close,
            message,
        );
        dialog.set_secondary_text(Some(reason));
        dialog.set_modal(true);
        dialog.show();
        dialog.close();
    }

    fn do_erase(&self) {
        let device = self.device_manager.get_device();
        if device.is_none() {
            log::error!("nodriver");
            post_event(
                &self.sender,
                MgAction::DoneErase(Err(drivers::Error::NoDriver)),
            );
            return;
        }
        let d = device.unwrap();
        let sender = self.sender.clone();
        print_on_err!(thread::Builder::new().name("eraser".into()).spawn(move || {
            post_event(
                &sender,
                if d.open() {
                    match d.erase() {
                        Ok(_) => {
                            log::debug!("success erasing");
                            MgAction::DoneErase(Ok(()))
                        }
                        Err(e) => MgAction::DoneErase(Err(e)),
                    }
                } else {
                    MgAction::DoneErase(Err(drivers::Error::Failed(i18n("Open failed."))))
                },
            );
        }));
    }

    fn settings_dir() -> path::PathBuf {
        // XXX replace this by glib stuff when we can.
        // Also we treat a failure of this as fatal.
        let mut path: path::PathBuf = dirs::home_dir().expect("Can't locate home_dir");
        path.push(".gpsami");
        path
    }

    fn save_settings(&self) -> Result<(), glib::Error> {
        let mut path = Self::settings_dir();
        path.push("gpsami.ini");
        self.prefs_store.save_to_file(path.to_str().unwrap())
    }

    fn set_output_destination_dir(&mut self, output: &path::Path) {
        self.output_dest_dir = output.to_owned();
    }

    pub fn load_settings(&mut self) -> Result<(), glib::Error> {
        let mut path = Self::settings_dir();
        if let Err(e) = std::fs::create_dir_all(path.clone()) {
            return Err(glib::Error::new(
                glib::FileError::Failed,
                &format!("Can't create settings dir '{path:?}': {e}"),
            ));
        }
        path.push("gpsami.ini");

        if let Err(e) = self
            .prefs_store
            .load_from_file(path, glib::KeyFileFlags::NONE)
        {
            log::error!("error with g_key_file {e}");
            Err(e)
        } else {
            Ok(())
        }
    }

    /// Start the app.
    pub fn start(&mut self) {
        utils::setup_text_combo(&self.model_combo, &self.model_store);
        utils::setup_text_combo(&self.port_combo, &self.port_store);
        self.populate_model_combo();
        if let Some(window) = self.gapp.window_by_id(self.window_id) {
            window.present();
        }
    }

    /// Rescan devices. On start and when new device is connected.
    fn rescan_devices(&mut self) {
        self.populate_model_combo();
    }

    fn populate_port_combo(&mut self, ports: &[drivers::Port]) {
        self.port_store.clear();
        for port in ports {
            log::debug!("adding port {port:?}");
            utils::add_text_row(&self.port_store, port.path.to_str().unwrap(), &port.id);
        }
    }

    fn populate_model_combo(&mut self) {
        self.model_store.clear();
        {
            let devices = self.device_manager.devices_desc();
            for dev in devices {
                utils::add_text_row(&self.model_store, &dev.id, &dev.label);
            }
        }

        if let Ok(model) = self.prefs_store.string("device", "model") {
            self.model_combo.set_active_id(Some(model.as_ref()));
        }

        if let Ok(port) = self.prefs_store.string("device", "port") {
            self.port_combo.set_active_id(Some(port.as_ref()));
        }
    }

    fn model_changed(&mut self, id: &str) {
        log::debug!("model changed to {id}");
        self.prefs_store.set_string("device", "model", id);
        if self.save_settings().is_err() {
            log::error!("Error loading settings");
        }

        let cap = self.device_manager.device_capability(id);
        if let Some(cap) = cap {
            self.update_device_capability(&cap);
            self.device_manager.set_model(id);
            let ports = self.device_manager.get_ports_for_model(id);
            self.populate_port_combo(&ports.unwrap_or_default());
        } else {
            // XXX clear device.
        }
    }

    fn update_device_capability(&self, capability: &devices::Capability) {
        self.erase_checkbtn.set_sensitive(capability.can_erase);
        if let Some(a) = self
            .gapp
            .window_by_id(self.window_id)
            .and_then(|w| w.downcast::<gtk::ApplicationWindow>().ok())
            .and_then(|w| w.lookup_action("erase"))
        {
            if let Ok(sa) = a.downcast::<gio::SimpleAction>() {
                sa.set_enabled(capability.can_erase_only);
            }
        }
    }

    fn port_changed(&mut self, id: &str) {
        self.prefs_store.set_string("device", "port", id);
        if self.save_settings().is_err() {
            log::error!("Error saving settings");
        }

        self.device_manager.set_port(id);

        if let Some(a) = self
            .gapp
            .window_by_id(self.window_id)
            .and_then(|w| w.downcast::<gtk::ApplicationWindow>().ok())
            .and_then(|w| w.lookup_action("download"))
        {
            if let Ok(sa) = a.downcast::<gio::SimpleAction>() {
                sa.set_enabled(!id.is_empty());
            }
        }
    }

    fn set_state(&mut self, state: UiState) {
        match state {
            UiState::Idle => {
                self.content_box.set_sensitive(true);
            }
            UiState::InProgress => {
                self.content_box.set_sensitive(false);
            }
        }
    }

    pub fn process_event(&mut self, evt: MgAction) {
        match evt {
            MgAction::RescanDevices => {
                self.rescan_devices();
            }
            MgAction::ModelChanged(ref id) => {
                self.model_changed(id);
            }
            MgAction::PortChanged(ref id) => self.port_changed(id),
            MgAction::StartErase => {
                self.set_state(UiState::InProgress);
                self.do_erase();
            }
            MgAction::DoneErase(e) => {
                match e {
                    Ok(_) => self
                        .toast_overlay
                        .add_toast(adw::Toast::new(&i18n("Erase finished."))),
                    Err(drivers::Error::Cancelled) => self
                        .toast_overlay
                        .add_toast(adw::Toast::new(&i18n("Erase cancelled."))),
                    Err(e) => self.report_error(&i18n("Error erasing GPS data."), &e.to_string()),
                }
                self.set_state(UiState::Idle);
            }
            MgAction::StartDownload => {
                self.set_state(UiState::InProgress);
                self.do_download();
            }
            MgAction::DoneDownload(e) => {
                log::debug!("done download {e:?}");
                match e {
                    Ok(_) => self
                        .toast_overlay
                        .add_toast(adw::Toast::new(&i18n("Download finished."))),
                    Err(drivers::Error::Cancelled) => self
                        .toast_overlay
                        .add_toast(adw::Toast::new(&i18n("Download cancelled."))),
                    Err(e) => {
                        self.report_error(&i18n("Error downloading GPS data."), &e.to_string())
                    }
                }
                self.set_state(UiState::Idle);
            }
            MgAction::SetOutputDir(f) => {
                self.set_output_destination_dir(f.as_ref());
                self.prefs_store
                    .set_string("output", "dir", f.to_str().unwrap());
                if self.save_settings().is_err() {
                    log::error!("Error saving settings");
                }
            }
        }
    }
}

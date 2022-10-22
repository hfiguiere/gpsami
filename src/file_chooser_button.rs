//
// (c) 2021 Hubert Figuière
//

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use gtk4::gio;
use gtk4::glib;
use gtk4::glib::subclass::Signal;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;

/// Print a message on error returned.
macro_rules! print_on_err {
    ($e:expr) => {
        if let Err(err) = $e {
            eprintln!(
                "{}:{} Error '{}': {}",
                file!(),
                line!(),
                stringify!($e),
                err
            );
        }
    };
}

glib::wrapper! {
    pub struct FileChooserButton(
        ObjectSubclass<FileChooserButtonPriv>)
        @extends gtk4::Button, gtk4::Widget;
}

impl FileChooserButton {
    pub fn new() -> FileChooserButton {
        glib::Object::new(&[])
    }

    pub fn get_filename(&self) -> Option<PathBuf> {
        let priv_ = FileChooserButtonPriv::from_instance(self);
        priv_.file.borrow().as_ref().and_then(|f| f.path())
    }

    pub fn set_filename<P: AsRef<Path>>(&self, f: P) {
        let file = gio::File::for_path(f.as_ref());
        self.set_property("file", &file);
    }
}

impl Default for FileChooserButton {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
pub struct FileChooserButtonPriv {
    file: RefCell<Option<gio::File>>,
    dialog: RefCell<Option<gtk4::FileChooserNative>>,
}

impl ObjectImpl for FileChooserButtonPriv {
    fn constructed(&self) {
        self.parent_constructed();

        let obj = self.instance();
        obj.connect_clicked(move |b| {
            let file_chooser = {
                let mut builder = gtk4::builders::FileChooserNativeBuilder::new()
                    .modal(true)
                    .action(gtk4::FileChooserAction::Open);
                if let Some(ref window) = b.root().and_then(|r| r.downcast::<gtk4::Window>().ok()) {
                    builder = builder.transient_for(window);
                }
                builder.build()
            };
            let priv_ = FileChooserButtonPriv::from_instance(b);
            // We must hold a reference to the Native dialog, or it crashes.
            priv_.dialog.replace(Some(file_chooser.clone()));
            let file = priv_.file.borrow().as_ref().and_then(|f| f.parent());
            print_on_err!(file_chooser.set_current_folder(file.as_ref()));

            file_chooser.connect_response(glib::clone!(@weak b => move |w, r| {
                if r == gtk4::ResponseType::Accept {
                    b.set_property("file", &w.file());
                    b.emit_by_name::<()>("file-set", &[]);
                }
                let priv_ = FileChooserButtonPriv::from_instance(&b);
                priv_.dialog.replace(None);
            }));
            file_chooser.show();
        });
    }

    fn properties() -> &'static [glib::ParamSpec] {
        use once_cell::sync::Lazy;
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![glib::ParamSpecObject::new(
                "file",
                "File",
                "The chosen file",
                gio::File::static_type(),
                glib::ParamFlags::READWRITE,
            )]
        });
        PROPERTIES.as_ref()
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "file" => {
                let file = value.get::<gio::File>().ok();
                self.file.replace(file.clone());
                if let Some(name) = file.as_ref().and_then(|f| f.basename()) {
                    self.instance().set_label(&name.to_string_lossy());
                }
            }
            _ => unimplemented!(),
        }
    }

    fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "file" => self.file.borrow().to_value(),
            _ => unimplemented!(),
        }
    }

    fn signals() -> &'static [Signal] {
        use once_cell::sync::Lazy;
        static SIGNALS: Lazy<Vec<Signal>> =
            Lazy::new(|| vec![Signal::builder("file-set").run_last().build()]);
        SIGNALS.as_ref()
    }
}

impl WidgetImpl for FileChooserButtonPriv {}
impl ButtonImpl for FileChooserButtonPriv {}

#[glib::object_subclass]
impl ObjectSubclass for FileChooserButtonPriv {
    const NAME: &'static str = "FileChooserButton";
    type Type = FileChooserButton;
    type ParentType = gtk4::Button;
}

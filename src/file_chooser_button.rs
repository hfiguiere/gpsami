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
        glib::Object::new(&[]).expect("Failed to create FileChooserButton")
    }

    pub fn get_filename(&self) -> Option<PathBuf> {
        let priv_ = FileChooserButtonPriv::from_instance(self);
        priv_.file.borrow().as_ref().and_then(|f| f.get_path())
    }

    pub fn set_filename<P: AsRef<Path>>(&self, f: P) {
        let file = gio::File::new_for_path(f.as_ref());
        print_on_err!(self.set_property("file", &file));
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
    fn constructed(&self, obj: &Self::Type) {
        self.parent_constructed(obj);

        obj.connect_clicked(move |b| {
            let file_chooser = {
                let mut builder = gtk4::FileChooserNativeBuilder::new()
                    .modal(true)
                    .action(gtk4::FileChooserAction::Open);
                if let Some(ref window) =
                    b.get_root().and_then(|r| r.downcast::<gtk4::Window>().ok())
                {
                    builder = builder.transient_for(window);
                }
                builder.build()
            };
            let priv_ = FileChooserButtonPriv::from_instance(b);
            // We must hold a reference to the Native dialog, or it crashes.
            priv_.dialog.replace(Some(file_chooser.clone()));
            if let Some(ref file) = priv_.file.borrow().as_ref().and_then(|f| f.get_parent()) {
                print_on_err!(file_chooser.set_current_folder(file));
            }
            file_chooser.connect_response(glib::clone!(@weak b => move |w, r| {
                if r == gtk4::ResponseType::Accept {
                    print_on_err!(b.set_property("file", &w.get_file()));
                    print_on_err!(b.emit_by_name("file-set", &[]));
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
            vec![glib::ParamSpec::object(
                "file",
                "File",
                "The chosen file",
                gio::File::static_type(),
                glib::ParamFlags::READWRITE,
            )]
        });
        PROPERTIES.as_ref()
    }

    fn set_property(
        &self,
        obj: &Self::Type,
        _id: usize,
        value: &glib::Value,
        pspec: &glib::ParamSpec,
    ) {
        match pspec.get_name() {
            "file" => {
                let file = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.file.replace(file.clone());
                if let Some(name) = file.as_ref().and_then(|f| f.get_basename()) {
                    obj.set_label(&name.to_string_lossy());
                }
            }
            _ => unimplemented!(),
        }
    }

    fn get_property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.get_name() {
            "file" => self.file.borrow().to_value(),
            _ => unimplemented!(),
        }
    }

    fn signals() -> &'static [Signal] {
        use once_cell::sync::Lazy;
        static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
            vec![Signal::builder("file-set", &[], <()>::static_type().into())
                .run_last()
                .build()]
        });
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

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

extern crate dirs_next as dirs;
#[macro_use]
extern crate gtk_macros;
extern crate gudev;
extern crate libudev;

use adw::prelude::*;
use gettextrs::*;
use gtk4 as gtk;
use gtk4::gio;
use gtk4::glib;

#[macro_use]
mod utils;

use mgapplication::MgApplication;

mod config;
mod devices;
mod drivers;
mod file_chooser_button;
mod gpsbabel;
mod mgapplication;
mod static_resources;

pub enum Format {
    None,
    Gpx,
    Kml,
}

/// Init Gtk and stuff.
fn init() {
    use std::sync::Once;

    static START: Once = Once::new();

    START.call_once(|| {
        env_logger::init();

        glib::set_prgname(Some("gpsami"));

        // run initialization here
        gtk::init().expect("Failed to initialize GTK.");

        file_chooser_button::FileChooserButton::static_type();

        setlocale(LocaleCategory::LcAll, "");
        bindtextdomain("gpsami", config::LOCALEDIR).expect("Coudln't bind textdomain");
        textdomain("gpsami").expect("Couldn't find text domain");

        static_resources::init().expect("Could not load resources");
    });
}

fn main() {
    init();

    let gapp = adw::Application::new(
        Some("net.figuiere.gpsami"),
        gio::ApplicationFlags::FLAGS_NONE,
    );

    gapp.connect_activate(move |gapp| {
        let app = MgApplication::new(gapp);

        action!(
            gapp,
            "quit",
            glib::clone!(
                #[weak]
                gapp,
                move |_, _| {
                    gapp.quit();
                }
            )
        );

        app.borrow_mut().start();
    });

    let ret = gapp.run();
    std::process::exit(ret.into());
}

#[test]
fn it_works() {}

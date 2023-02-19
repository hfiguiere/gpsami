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

use gtk4 as gtk;
use gtk4::prelude::*;

/// Print a message on error returned.
#[macro_export]
macro_rules! print_on_err {
    ($e:expr) => {
        if let Err(err) = $e {
            log::error!(
                "{}:{} Error '{}': {}",
                file!(),
                line!(),
                stringify!($e),
                err
            );
        }
    };
}

/// Setup a gtk::ComboBox model to have id in column 1 and label in column 2
/// All text.
pub fn setup_text_combo(combo: &gtk::ComboBox, model: &gtk::ListStore) {
    combo.set_model(Some(model));
    combo.set_id_column(0);
    combo.set_entry_text_column(1);

    let cell = gtk::CellRendererText::new();
    combo.pack_start(&cell, true);
    combo.add_attribute(&cell, "text", 1);
}

/// Add a row with two text column into the list store.
pub fn add_text_row(store: &gtk::ListStore, col1: &str, col2: &str) -> gtk::TreeIter {
    store.insert_with_values(None, &[(0, &String::from(col1)), (1, &String::from(col2))])
}

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

use std::fmt;
use std::io;
use std::path::PathBuf;

use serde::Deserialize;

use crate::Format;

#[derive(Debug)]
pub struct Port {
    pub id: String,
    pub label: String,
    pub path: PathBuf,
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub enum PortType {
    None,
    UsbSerial,
    RfComm, // Bluetooth Serial
}

#[derive(Clone, Debug, Deserialize)]
pub struct Desc {
    pub id: String,
    // the port to look for.
    pub ports: Vec<PortType>,
}

pub enum Error {
    Success,
    Unsupported,
    NoDriver,
    Cancelled,
    WrongArg,
    Failed(String),
    IoError(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Success => write!(f, "Success"),
            Error::Unsupported => write!(f, "Unsupported"),
            Error::NoDriver => write!(f, "No driver"),
            Error::Cancelled => write!(f, "Cancelled"),
            Error::WrongArg => write!(f, "WrongArg"),
            Error::Failed(ref s) => write!(f, "{s}"),
            Error::IoError(ref e) => write!(f, "{e}"),
        }
    }
}

pub trait Driver {
    /// open the device
    fn open(&mut self) -> bool;
    /// close the device
    fn close(&mut self) -> bool;
    /// Download the track in specified format
    /// Return the PathBuf pointing to the datafile.
    fn download(&self, format: Format, erase: bool) -> Result<PathBuf, Error>;
    /// Erase the tracks
    fn erase(&self) -> Result<(), Error>;
}

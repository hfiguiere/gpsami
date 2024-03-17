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

use std::io;
use std::path::PathBuf;

use serde::Deserialize;
use thiserror::Error;

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

#[derive(Error, Debug)]
pub enum Error {
    #[error("Unsupported")]
    Unsupported,
    #[error("No driver")]
    NoDriver,
    #[error("Cancelled")]
    Cancelled,
    #[error("Incorrect argument")]
    WrongArg,
    #[error("Failed: {0}")]
    Failed(String),
    #[error("IO error {0}")]
    Io(#[from] io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait Driver {
    /// open the device
    fn open(&self) -> bool;
    /// close the device
    fn close(&self) -> bool;
    /// Download the track in specified format
    /// Return the PathBuf pointing to the datafile.
    fn download(&self, format: Format, erase: bool) -> Result<PathBuf>;
    /// Erase the tracks
    fn erase(&self) -> Result<()>;
}

//
// (c) 2016-2024 Hubert Figuière
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

use std::path::PathBuf;
use std::process::Command;

use tempfile::TempDir;

use crate::devices::Capability;
use crate::drivers::Driver;
use crate::drivers::Error;
use crate::Format;

/// GpsBabel "driver". Will use gpsbabel to connect to device.
pub struct GpsBabel {
    device_id: String,
    port: String,
    cap: Capability,
}

impl GpsBabel {
    pub fn new(device: String, port: &str, capability: Capability) -> Self {
        GpsBabel {
            device_id: device,
            port: port.to_owned(),
            cap: capability,
        }
    }

    /// Return a string associated with the format.
    /// Or None
    fn format_to_string(format: &Format) -> Option<&'static str> {
        match *format {
            Format::Gpx => Some("gpx"),
            Format::Kml => Some("kml"),
            _ => None,
        }
    }

    /// Return an extension (with .) associated with the format.
    /// Or None
    fn format_to_extension(format: &Format) -> Option<&'static str> {
        match *format {
            Format::Gpx => Some(".gpx"),
            Format::Kml => Some(".kml"),
            _ => None,
        }
    }

    /// Build the basic command line for the device on port, eventually for delete
    /// after download or erase only.
    fn build_basic_command_line(
        device_id: &str,
        port: &str,
        erase: bool,
        erase_only: bool,
    ) -> Command {
        let mut device_string = String::from(device_id);
        // erase and erase_only can't be true at the same time.
        // assume that if erase is true, then we want a download.
        if erase {
            device_string.push_str(",erase");
        } else if erase_only {
            device_string.push_str(",erase_only");
        }
        let mut command = Command::new("gpsbabel");
        command
            .arg("-t")
            .arg("-w")
            .arg("-i")
            .arg(device_string)
            .arg("-f")
            .arg(port);

        command
    }
}

impl Driver for GpsBabel {
    fn open(&self) -> bool {
        !self.port.is_empty()
    }

    fn close(&self) -> bool {
        true
    }

    /// Download the data into a file. Return the PathBuf to said file on success.
    /// Caller is responsible for deleting the file.
    fn download(&self, format: Format, erase: bool, tempdir: &TempDir) -> Result<PathBuf, Error> {
        // we requested erase at the same time and it is not supported.
        if erase && !self.cap.can_erase {
            return Err(Error::Unsupported);
        }

        let fmt_string_opt = Self::format_to_string(&format);
        if fmt_string_opt.is_none() {
            // invalid format
            return Err(Error::WrongArg);
        }
        let fmt_string = fmt_string_opt.unwrap();

        let extension_opt = Self::format_to_extension(&format);
        if extension_opt.is_none() {
            // invalid format
            return Err(Error::WrongArg);
        }
        let extension = extension_opt.unwrap();

        let outfile = tempdir.path().join(String::from("gpsami") + extension);

        /* gpsbabel -t -w -i m241 -f /dev/ttyACM0 -o gpx -F $1 */
        let output = GpsBabel::build_basic_command_line(&self.device_id, &self.port, erase, false)
            .arg("-o")
            .arg(fmt_string) // format
            .arg("-F")
            .arg(String::from(outfile.to_str().unwrap()))
            .output()?;
        log::debug!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        if !output.status.success() {
            let err_output = String::from_utf8_lossy(&output.stderr);
            log::error!("{}: {}", output.status, err_output);
            return Err(Error::Failed(err_output.into_owned()));
        }
        Ok(outfile)
    }

    /// Erase the logs on the device. Return an error if not capable.
    fn erase(&self) -> Result<(), Error> {
        // Device doesn't support "erase only"
        if !self.cap.can_erase_only {
            return Err(Error::Unsupported);
        }
        /* gpsbabel -t -w -i m241,erase_only -f /dev/ttyACM0 */
        let output = GpsBabel::build_basic_command_line(&self.device_id, &self.port, false, true)
            .output()?;
        log::debug!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        if !output.status.success() {
            let err_output = String::from_utf8_lossy(&output.stderr);
            log::error!("{}: {}", output.status, err_output);
            return Err(Error::Failed(err_output.into_owned()));
        }
        Ok(())
    }
}

#[test]
fn test_command_builder() {
    let command = GpsBabel::build_basic_command_line("foo", "ttyS0", false, false);
    assert_eq!(
        format!("{:?}", command),
        "\"gpsbabel\" \"-t\" \"-w\" \"-i\" \"foo\" \"-f\" \"ttyS0\""
    )
}

#[test]
fn test_format() {
    let result = GpsBabel::format_to_string(&Format::Gpx);
    assert!(result.is_some());
    assert_eq!(result, Some("gpx"));

    let result = GpsBabel::format_to_string(&Format::None);
    assert!(result.is_none());
}

#[test]
fn test_extensions() {
    let result = GpsBabel::format_to_extension(&Format::Gpx);
    assert!(result.is_some());
    assert_eq!(result, Some(".gpx"));

    let result = GpsBabel::format_to_string(&Format::None);
    assert!(result.is_none());
}

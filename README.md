gpsami
======

gpsami is a small GUI application to download data from a GPS loggers
and save it as GPX.

It is written in Rust and uses Gtk4 for the UI and gpsbabel for the
download part.

Requires libudev for listing devices, therefor require some effort to
run on non-Linux. Patches welcome.

See [`doc/devices.md`](doc/devices.md) for information about device support.

To build
--------

### From the command line

You need meson and Rust to build. You also need gtk4 and gudev.

To configure the (release) build do:

````
$ meson build
````

If you want to build a development version, do:
````
$ meson -Dprofile=development build
````

To build

````
$ ninja -C build
````

### Using Builder

You can just open Builder and the flatpak manifest will be used to
build.

Note: since gpsami requires gpsbabel, and gpsbabel needs to be build,
we use the KDE sdk to satisfy the Qt requirement from gpsbabel.

License
-------

This software is licensed under the GNU Public License v3. See COPYING.

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.


Contributors
------------

Written and maintained by:

  Hubert Figuière <hub@figuiere.net>

Contributors:

  Johannes J. Schmidt <jo@die-tf.de>

Icon by:

  Daniel Galleguillos

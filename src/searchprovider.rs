// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Search provider implementation.

use gtk::gio::{DBusInterfaceInfo, DBusNodeInfo};

static SEARCH_PROVIDER_2_XML: &str = include_str!("../dbus-1/org.gnome.ShellSearchProvider2.xml");

pub static SEARCH_PROVIDER_2_IFACE_NAME: &str = "org.gnome.Shell.SearchProvider2";

pub fn search_provider_2_interface() -> DBusInterfaceInfo {
    // We unwrap here since we know that the XML is valid and contains the
    // desired interface, so none of this can realistically fail.
    DBusNodeInfo::for_xml(SEARCH_PROVIDER_2_XML)
        .unwrap()
        .lookup_interface(SEARCH_PROVIDER_2_IFACE_NAME)
        .unwrap()
}

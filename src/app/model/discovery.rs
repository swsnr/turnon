// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use glib::dpgettext2;
use gtk::gio;

use crate::net::arpcache::{
    self, ArpCacheEntry, ArpCacheEntryFlags, ArpHardwareType, ArpKnownHardwareType,
};

use super::Device;
use crate::config::G_LOG_DOMAIN;

/// Whether `entry` denotes a complete ethernet entry.
///
/// Return `true` if `entry` has the `ATF_COM` flag which signifies that the
/// entry is complete, and the `Ether` hardware type.
fn is_complete_ethernet_entry(entry: &ArpCacheEntry) -> bool {
    entry.hardware_type == ArpHardwareType::Known(ArpKnownHardwareType::Ether)
        && entry.flags.contains(ArpCacheEntryFlags::ATF_COM)
}

/// Read devices from the ARP cache.
///
/// Return an error if opening the ARP cache file failed; otherwise return a
/// (potentially empty) iterator of all devices found in the ARP cache, skipping
/// over invalid or malformed entries.
///
/// All discovered devices have their IP address has `host` and a constant
/// human readable and translated `label`.
pub async fn devices_from_arp_cache() -> std::io::Result<impl Iterator<Item = Device>> {
    let arp_cache = gio::spawn_blocking(arpcache::read_linux_arp_cache)
        .await
        .unwrap()?;

    Ok(arp_cache
        .filter_map(|item| {
            item.inspect_err(|error| {
                glib::warn!("Failed to parse ARP cache entry: {error}");
            })
            .ok()
        })
        .filter(is_complete_ethernet_entry)
        .map(|entry| {
            Device::new(
                &dpgettext2(None, "discovered-device.label", "Discovered device"),
                entry.hardware_address.into(),
                &entry.ip_address.to_string(),
            )
        }))
}

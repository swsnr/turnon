// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::path::Path;

use glib::{dpgettext2, Object};
use gtk::gio;

use crate::net::arpcache::*;

use super::Device;
use crate::config::G_LOG_DOMAIN;

glib::wrapper! {
    /// Device discovery.
    pub struct DeviceDiscovery(ObjectSubclass<imp::DeviceDiscovery>) @implements gio::ListModel;
}

impl Default for DeviceDiscovery {
    fn default() -> Self {
        Object::builder().build()
    }
}

mod imp {
    use gtk::gio;
    use gtk::gio::prelude::*;
    use gtk::gio::subclass::prelude::*;

    use std::{
        cell::{Cell, RefCell},
        path::PathBuf,
    };

    use super::{super::Device, devices_from_arp_cache};
    use crate::{config::G_LOG_DOMAIN, net::arpcache::default_arp_cache_path};

    #[derive(Debug, glib::Properties)]
    #[properties(wrapper_type = super::DeviceDiscovery)]
    pub struct DeviceDiscovery {
        #[property(get, set = Self::set_discovery_enabled)]
        discovery_enabled: Cell<bool>,
        #[property(get, set)]
        arp_cache_file: RefCell<PathBuf>,
        discovered_devices: RefCell<Vec<Device>>,
    }

    impl DeviceDiscovery {
        fn set_discovery_enabled(&self, enabled: bool) {
            self.discovery_enabled.replace(enabled);
            self.obj().notify_discovery_enabled();
            if enabled {
                self.scan_devices();
            } else {
                let mut discovered_devices = self.discovered_devices.borrow_mut();
                let n_items_removed = discovered_devices.len();
                discovered_devices.clear();
                // Drop mutable borrow of devices before emtting the signal, because signal handlers
                // can already try to access the mdoel
                drop(discovered_devices);
                self.obj()
                    .items_changed(0, n_items_removed.try_into().unwrap(), 0);
            }
        }

        fn scan_devices(&self) {
            let discovery = self.obj().clone();
            glib::spawn_future_local(async move {
                match devices_from_arp_cache(discovery.arp_cache_file()).await {
                    Ok(devices_from_arp_cache) => {
                        if discovery.discovery_enabled() {
                            // If discovery is still enabled remember all discovered devices
                            let mut devices = discovery.imp().discovered_devices.borrow_mut();
                            let len_before = devices.len();
                            devices.extend(devices_from_arp_cache);
                            let n_changed = devices.len() - len_before;
                            drop(devices);
                            discovery.items_changed(
                                len_before.try_into().unwrap(),
                                0,
                                n_changed.try_into().unwrap(),
                            );
                        }
                    }
                    Err(error) => {
                        glib::warn!("Failed to read ARP cache: {error}");
                    }
                }
            });
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DeviceDiscovery {
        const NAME: &'static str = "DeviceDiscovery";

        type Type = super::DeviceDiscovery;

        type Interfaces = (gio::ListModel,);

        fn new() -> Self {
            Self {
                discovery_enabled: Default::default(),
                arp_cache_file: RefCell::new(default_arp_cache_path().into()),
                discovered_devices: Default::default(),
            }
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for DeviceDiscovery {}

    impl ListModelImpl for DeviceDiscovery {
        fn item_type(&self) -> glib::Type {
            Device::static_type()
        }

        fn n_items(&self) -> u32 {
            self.discovered_devices.borrow().len().try_into().unwrap()
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.discovered_devices
                .borrow()
                .get(usize::try_from(position).unwrap())
                .map(|d| d.clone().upcast())
        }
    }
}

/// Read devices from the ARP cache.
///
/// Return an error if opening the ARP cache file failed; otherwise return a
/// (potentially empty) iterator of all devices found in the ARP cache, skipping
/// over invalid or malformed entries.
///
/// All discovered devices have their IP address has `host` and a constant
/// human readable and translated `label`.
async fn devices_from_arp_cache<P: AsRef<Path> + Send + 'static>(
    arp_cache_file: P,
) -> std::io::Result<impl Iterator<Item = Device>> {
    let arp_cache = gio::spawn_blocking(move || read_arp_cache_from_path(arp_cache_file))
        .await
        .unwrap()?;

    Ok(arp_cache
        .filter_map(|item| {
            item.inspect_err(|error| {
                glib::warn!("Failed to parse ARP cache entry: {error}");
            })
            .ok()
        })
        // Only consider ethernet devices
        .filter(|entry| entry.hardware_type == ArpHardwareType::Known(ArpKnownHardwareType::Ether))
        // Only include complete ARP cache entries, where the hardware address is fully known and valid
        .filter(|entry| entry.flags.contains(ArpCacheEntryFlags::ATF_COM))
        .map(|entry| {
            Device::new(
                &dpgettext2(None, "discovered-device.label", "Discovered device"),
                entry.hardware_address.into(),
                &entry.ip_address.to_string(),
            )
        }))
}

// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::time::Duration;

use futures_util::{select_biased, FutureExt};
use glib::prelude::*;
use glib::subclass::types::ObjectSubclass;
use glib::{object::IsA, subclass::types::IsImplementable};
use gtk::gio::IOErrorEnum;

use crate::net::MacAddr6Boxed;
use crate::{config::G_LOG_DOMAIN, net::wol};

glib::wrapper! { pub struct WakeableDevice(ObjectInterface<imp::WakeableDevice>); }

pub trait WakeableDeviceExt: IsA<WakeableDevice> {
    /// The human-readable label for this device.
    fn label(&self) -> String;

    /// The MAC address of this device.
    fn mac_address(&self) -> MacAddr6Boxed;

    /// Send a magic packet to this device.
    async fn wol(&self) -> Result<(), glib::Error> {
        let mac_address = self.mac_address();
        glib::info!(
            "Sending magic packet for mac address {mac_address} of device {}",
            self.label()
        );
        let wol_timeout = Duration::from_secs(5);
        let result = select_biased! {
            result = wol(*mac_address).fuse() => result,
            _ = glib::timeout_future(wol_timeout).fuse() => {
                let message = &format!("Failed to send magic packet within {wol_timeout:#?}");
                Err(glib::Error::new(IOErrorEnum::TimedOut, message))
            }
        };
        result
            .inspect(|_| {
                glib::info!(
                    "Sent magic packet to {mac_address} of device {}",
                    self.label()
                );
            })
            .inspect_err(|error| {
                glib::warn!(
                    "Failed to send magic packet to {mac_address} of device{}: {error}",
                    self.label()
                );
            })
    }
}

impl<T: IsA<WakeableDevice>> WakeableDeviceExt for T {
    fn mac_address(&self) -> MacAddr6Boxed {
        self.property("mac-address")
    }

    fn label(&self) -> String {
        self.property("label")
    }
}

unsafe impl<T: ObjectSubclass> IsImplementable<T> for WakeableDevice {}

mod imp {
    use std::sync::OnceLock;

    use glib::prelude::*;
    use glib::subclass::prelude::*;
    use glib::ParamSpecString;

    use crate::net::MacAddr6Boxed;

    #[repr(C)]
    #[derive(Copy, Clone, Debug)]
    pub struct WakeableDeviceClass(glib::gobject_ffi::GTypeInterface);

    unsafe impl InterfaceStruct for WakeableDeviceClass {
        type Type = WakeableDevice;
    }

    pub struct WakeableDevice;

    #[glib::object_interface]
    impl ObjectInterface for WakeableDevice {
        const NAME: &'static str = "WakeableDevice";
        type Interface = WakeableDeviceClass;

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                vec![
                    ParamSpecString::builder("label").read_only().build(),
                    ParamSpecString::builder("host").read_only().build(),
                    MacAddr6Boxed::param_spec_builder()("mac-address")
                        .read_only()
                        .build(),
                ]
            })
        }
    }
}

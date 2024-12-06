// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::borrow::Cow;
use std::fmt::Display;
use std::net::IpAddr;
use std::time::Duration;

use futures_util::stream::{FuturesOrdered, FuturesUnordered};
use futures_util::StreamExt;
use futures_util::{select_biased, FutureExt};
use gtk::gio;
use gtk::gio::IOErrorEnum;
use gtk::prelude::*;
use macaddr::MacAddr6;

use crate::config;
use crate::net::{ping_address_with_timeout, resolve_target};

use super::model::Device;

#[derive(Debug)]
pub enum DevicePingResult {
    ResolveFailed(glib::Error),
    Pinged(Vec<(IpAddr, Result<Duration, glib::Error>)>),
}

fn timeout_err(timeout: Duration) -> glib::Error {
    glib::Error::new(
        IOErrorEnum::TimedOut,
        &format!("Timeout after {}ms", timeout.as_millis()),
    )
}

async fn ping_device(device: Device) -> (Device, DevicePingResult) {
    // For debug info we use a very aggressive timeout for resolution and pings.
    // We expect everything to be in the local network anyways.
    let timeout = Duration::from_millis(500);
    let addresses = select_biased! {
        addresses = resolve_target(device.host().into()).fuse() => addresses,
        _ = glib::timeout_future(timeout).fuse() => Err(timeout_err(timeout)),
    };

    match addresses {
        Err(error) => (device, DevicePingResult::ResolveFailed(error)),
        Ok(addresses) => {
            let pings = addresses
                .into_iter()
                .map(|addr| ping_address_with_timeout(addr, 1, timeout).map(move |r| (addr, r)))
                .collect::<FuturesUnordered<_>>()
                .collect::<Vec<_>>()
                .await;

            (device, DevicePingResult::Pinged(pings))
        }
    }
}

#[derive(Debug)]
pub struct DebugInfo {
    /// The application ID we're running under.
    ///
    /// Differentiate between the nightly devel package, and the released version.
    pub app_id: &'static str,
    /// The version.
    pub version: &'static str,
    /// Whether the application runs inside a flatpak sandbox.
    pub flatpak: bool,
    /// Overall network connectivity
    pub connectivity: gio::NetworkConnectivity,
    /// Results from pinging devices once, for debugging.
    pub ping_results: Vec<(Device, DevicePingResult)>,
}

impl DebugInfo {
    /// Assemble debug information for Turn On.
    ///
    /// This method returns a human-readable plain text debug report which can help
    /// to identify issues.
    pub async fn assemble(model: gio::ListStore) -> DebugInfo {
        let monitor = gio::NetworkMonitor::default();
        let (connectivity, ping_results) = futures_util::future::join(
            // Give network monitor time to actually figure out what the state of the network is,
            // especially inside a flatpak sandbox, see https://gitlab.gnome.org/GNOME/glib/-/issues/1718
            glib::timeout_future(Duration::from_millis(500)).map(|_| monitor.connectivity()),
            std::iter::once(Device::new("localhost", MacAddr6::nil(), "localhost"))
                .chain(model.into_iter().map(|d| d.unwrap().downcast().unwrap()))
                .map(ping_device)
                .collect::<FuturesOrdered<_>>()
                .collect::<Vec<_>>(),
        )
        .await;
        DebugInfo {
            app_id: config::APP_ID,
            version: config::VERSION,
            flatpak: config::running_in_flatpak(),
            connectivity,
            ping_results,
        }
    }

    pub fn suggested_file_name(&self) -> String {
        format!("{}-{}-debug.txt", self.app_id, self.version)
    }
}

impl Display for DebugInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let connectivity: Cow<'static, str> = match self.connectivity {
            gio::NetworkConnectivity::Local => "Local".into(),
            gio::NetworkConnectivity::Limited => "Limited".into(),
            gio::NetworkConnectivity::Portal => "Portal".into(),
            gio::NetworkConnectivity::Full => "Full".into(),
            other => format!("Other {:?}", other).into(),
        };
        let pings = self
            .ping_results
            .iter()
            .map(|(d, r)| match r {
                DevicePingResult::ResolveFailed(error) => {
                    format!("Host {}\n    Failed to resolve: {error}", d.host())
                }
                DevicePingResult::Pinged(addresses) => {
                    format!(
                        "Host {}:\n{}",
                        d.host(),
                        addresses
                            .iter()
                            .map(|(addr, result)| {
                                format!(
                                    "    {addr}: {}",
                                    result
                                        .as_ref()
                                        .map(|d| format!("{}ms", d.as_millis()))
                                        .unwrap_or_else(|error| error.to_string())
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        writeln!(
            f,
            "DEBUG REPORT {} {}

THIS REPORT CONTAINS HOST NAMES AND IP ADDRESSES OF YOUR DEVICES.

IF YOU CONSIDER THIS REPORT SENSITIVE DO NOT POST IT PUBLICLY!

Flatpak? {}
Network connectivity: {connectivity}

{}",
            self.app_id, self.version, self.flatpak, pings
        )?;
        Ok(())
    }
}

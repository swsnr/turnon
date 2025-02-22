// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::borrow::Cow;
use std::fmt::Display;
use std::net::IpAddr;
use std::time::Duration;

use futures_util::stream::{FuturesOrdered, FuturesUnordered};
use futures_util::FutureExt;
use futures_util::StreamExt;
use gtk::gio;
use gtk::prelude::*;
use macaddr::MacAddr6;

use crate::config;
use crate::futures::future_with_timeout;
use crate::net::arpcache::{default_arp_cache_path, read_arp_cache_from_path, ArpCacheEntry};
use crate::net::{ping_address, PingDestination};

use super::model::{Device, Devices};

#[derive(Debug)]
pub enum DevicePingResult {
    ResolveFailed(glib::Error),
    Pinged(Vec<(IpAddr, Result<Duration, glib::Error>)>),
}

async fn ping_device(device: Device) -> (Device, DevicePingResult) {
    // For debug info we use a very aggressive timeout for resolution and pings.
    // We expect everything to be in the local network anyways.
    let timeout = Duration::from_millis(500);
    let destination = PingDestination::from(device.host());

    match future_with_timeout(timeout, destination.resolve()).await {
        Err(error) => (device, DevicePingResult::ResolveFailed(error)),
        Ok(addresses) => {
            let pings = addresses
                .into_iter()
                .map(|addr| {
                    future_with_timeout(timeout, ping_address(addr, 1)).map(move |r| (addr, r))
                })
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
    /// Raw contents of ARP cache.
    pub arp_cache_contents: std::io::Result<String>,
    /// Parsed contents of ARP cache.
    pub parsed_arp_cache: std::io::Result<Vec<ArpCacheEntry>>,
}

impl DebugInfo {
    /// Assemble debug information for Turn On.
    ///
    /// This method returns a human-readable plain text debug report which can help
    /// to identify issues.
    pub async fn assemble(devices: Devices) -> DebugInfo {
        let monitor = gio::NetworkMonitor::default();
        let (connectivity, ping_results) = futures_util::future::join(
            // Give network monitor time to actually figure out what the state of the network is,
            // especially inside a flatpak sandbox, see https://gitlab.gnome.org/GNOME/glib/-/issues/1718
            glib::timeout_future(Duration::from_millis(500)).map(|()| monitor.connectivity()),
            std::iter::once(Device::new(
                "localhost",
                MacAddr6::nil().into(),
                "localhost",
            ))
            .chain(
                devices
                    .registered_devices()
                    .into_iter()
                    .map(|d| d.unwrap().downcast().unwrap()),
            )
            .map(ping_device)
            .collect::<FuturesOrdered<_>>()
            .collect::<Vec<_>>(),
        )
        .await;
        let arp_cache_contents =
            gio::spawn_blocking(|| std::fs::read_to_string(default_arp_cache_path()))
                .await
                .unwrap();
        let parsed_arp_cache = gio::spawn_blocking(|| {
            read_arp_cache_from_path(default_arp_cache_path()).and_then(Iterator::collect)
        })
        .await
        .unwrap();
        DebugInfo {
            app_id: config::APP_ID,
            version: config::CARGO_PKG_VERSION,
            flatpak: config::running_in_flatpak(),
            connectivity,
            ping_results,
            arp_cache_contents,
            parsed_arp_cache,
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
            other => format!("Other {other:?}").into(),
        };
        let pings =
            self.ping_results
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
                                        result.as_ref().map_or_else(
                                            ToString::to_string,
                                            |d| format!("{}ms", d.as_millis())
                                        )
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join("\n")
                        )
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
        let arp_cache_contents = match &self.arp_cache_contents {
            Ok(contents) => Cow::Borrowed(contents),
            Err(error) => Cow::Owned(format!("Failed: {error}")),
        };
        let parsed_arp_cache = match &self.parsed_arp_cache {
            Ok(entries) => entries
                .iter()
                .map(|entry| format!("{entry:?}"))
                .collect::<Vec<_>>()
                .join("\n"),
            Err(error) => format!("Failed: {error}"),
        };
        writeln!(
            f,
            "DEBUG REPORT {} {}

THIS REPORT CONTAINS HOST NAMES, IP ADDRESSES, AND HARDWARE ADDRESSES OF YOUR DEVICES.

IF YOU CONSIDER THIS REPORT SENSITIVE DO NOT POST IT PUBLICLY!

Flatpak? {}
Network connectivity: {connectivity}

{pings}

ARP cache contents:
{arp_cache_contents}

Parsed ARP cache:
{parsed_arp_cache}",
            self.app_id, self.version, self.flatpak,
        )?;
        Ok(())
    }
}

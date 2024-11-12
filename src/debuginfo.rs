// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt::Display;
use std::net::IpAddr;
use std::time::Duration;
use std::time::Instant;

use futures_util::stream::FuturesOrdered;
use futures_util::StreamExt;
use futures_util::{select_biased, FutureExt};
use gtk::gio;
use gtk::gio::IOErrorEnum;
use gtk::prelude::*;
use macaddr::MacAddr6;

use crate::config;
use crate::model::Device;
use crate::model::Devices;
use crate::net::Target;

#[derive(Debug)]
pub enum DevicePingResult {
    ResolveFailed(glib::Error),
    Pinged(Vec<(IpAddr, Result<Duration, glib::Error>)>),
}

fn timeout_err(timeout: Duration) -> glib::Error {
    glib::Error::new(
        IOErrorEnum::TimedOut,
        &format!("Timeout after {}s", timeout.as_secs()),
    )
}

async fn ping_device(device: Device) -> (Device, DevicePingResult) {
    let timeout = Duration::from_secs(2);
    let target = Target::from(device.host());

    let addresses = match target {
        Target::Dns(name) => {
            select_biased! {
                result = gio::Resolver::default().lookup_by_name_future(&name).fuse() => result,
                _ = glib::timeout_future(timeout).fuse() => Err(timeout_err(timeout)),
            }
        }
        Target::Addr(ip_addr) => Ok(vec![ip_addr.into()]),
    };

    match addresses {
        Err(error) => (device, DevicePingResult::ResolveFailed(error)),
        Ok(addresses) if addresses.is_empty() => {
            let error = glib::Error::new(
                IOErrorEnum::NotFound,
                &format!("No addresses found for {}", device.host()),
            );
            (device, DevicePingResult::ResolveFailed(error))
        }
        Ok(addresses) => {
            let ping_start = Instant::now();
            let pings = addresses
                .into_iter()
                .map(|addr| async move {
                    let addr = addr.into();
                    let ping = crate::net::ping(addr, 1);
                    select_biased! {
                        r = ping.fuse() => (addr, r.map(|_| Instant::now() - ping_start)),
                        _ = glib::timeout_future(timeout).fuse() => (addr, Err(timeout_err(timeout))),
                    }
                })
                .collect::<FuturesOrdered<_>>()
                .collect::<Vec<_>>().await;

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
    /// Results from pinging devices once, for debugging.
    pub ping_results: Vec<(Device, DevicePingResult)>,
}

impl DebugInfo {
    /// Assemble debug information for Turn On.
    ///
    /// This method returns a human-readable plain text debug report which can help
    /// to identify issues.
    pub async fn assemble(devices: Devices) -> DebugInfo {
        let ping_results = std::iter::once(Device::new(
            "localhost".to_owned(),
            MacAddr6::nil(),
            "localhost".to_owned(),
        ))
        .chain(devices.into_iter())
        .map(ping_device)
        .collect::<FuturesOrdered<_>>()
        .collect::<Vec<_>>()
        .await;
        DebugInfo {
            app_id: config::APP_ID,
            version: config::VERSION,
            flatpak: config::running_in_flatpak(),
            ping_results,
        }
    }

    pub fn suggested_file_name(&self) -> String {
        format!("{}-{}-debug.txt", self.app_id, self.version)
    }
}

impl Display for DebugInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
                        d.label(),
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

{}",
            self.app_id, self.version, self.flatpak, pings
        )?;
        Ok(())
    }
}
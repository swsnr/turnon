// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

use std::{borrow::Cow, future::Future, time::Duration};

use gio::prelude::*;
use glib::dpgettext2;
use gnome_app_utils::futures::{StreamExt, stream::FuturesOrdered};
use gtk::gio;

use crate::app::TurnOnApplication;
use crate::config::G_LOG_DOMAIN;
use crate::futures::future_with_timeout;
use crate::net::PingDestination;

use super::model::Device;

async fn turn_on_device(
    command_line: &gio::ApplicationCommandLine,
    device: &Device,
) -> glib::ExitCode {
    match device.wol().await {
        Ok(()) => {
            command_line.print_literal(
                &dpgettext2(
                    None,
                    "option.turn-on-device.message",
                    "Sent magic packet to mac address %1 of device %2\n",
                )
                .replace("%1", &device.mac_address().to_string())
                .replace("%2", &device.label()),
            );
            glib::ExitCode::SUCCESS
        }
        Err(error) => {
            command_line.printerr_literal(
                &dpgettext2(
                    None,
                    "option.turn-on-device.error",
                    "Failed to turn on device %1: %2\n",
                )
                .replace("%1", &device.label())
                .replace("%2", &error.to_string()),
            );
            glib::ExitCode::FAILURE
        }
    }
}

pub fn turn_on_device_by_label(
    app: &TurnOnApplication,
    command_line: &gio::ApplicationCommandLine,
    label: &str,
) -> glib::ExitCode {
    let guard = app.hold();
    glib::debug!("Turning on device in response to command line argument");
    let registered_devices = app.devices().registered_devices();
    let device = registered_devices
        .find_with_equal_func(|o| {
            o.downcast_ref::<Device>()
                .filter(|d| d.label() == label)
                .is_some()
        })
        .and_then(|position| registered_devices.item(position))
        .and_then(|o| o.downcast::<Device>().ok());
    if let Some(device) = device {
        glib::spawn_future_local(glib::clone!(
            #[strong]
            command_line,
            async move {
                let exit_code = turn_on_device(&command_line, &device).await;
                command_line.set_exit_status(exit_code.value());
                command_line.done();
                drop(guard);
            }
        ));
        glib::ExitCode::SUCCESS
    } else {
        command_line.printerr_literal(
            &dpgettext2(
                None,
                "option.turn-on-device.error",
                "No device found for label %s\n",
            )
            .replace("%s", label),
        );
        glib::ExitCode::FAILURE
    }
}

async fn ping_device(device: Device) -> (Device, Result<Duration, glib::Error>) {
    let destination = PingDestination::from(device.host());
    let result = future_with_timeout(Duration::from_millis(500), destination.ping(1)).await;
    (device, result.map(|v| v.1))
}

pub fn ping_all_devices<I: IntoIterator<Item = Device>>(
    devices: I,
) -> impl Future<Output = Vec<(Device, Result<Duration, glib::Error>)>> {
    devices
        .into_iter()
        .map(ping_device)
        .collect::<FuturesOrdered<_>>()
        .collect::<Vec<_>>()
}

pub fn list_devices(
    app: &TurnOnApplication,
    command_line: &gio::ApplicationCommandLine,
) -> glib::ExitCode {
    let guard = app.hold();
    glib::spawn_future_local(glib::clone!(
        #[strong]
        app,
        #[strong]
        command_line,
        async move {
            let pinged_devices = ping_all_devices(
                app.devices()
                    .registered_devices()
                    .into_iter()
                    .map(|o| o.unwrap().downcast().unwrap()),
            )
            .await;
            let (label_width, host_width) =
                pinged_devices.iter().fold((0, 0), |(lw, hw), (device, _)| {
                    (
                        lw.max(device.label().chars().count()),
                        hw.max(device.host().chars().count()),
                    )
                });
            for (device, result) in pinged_devices {
                let (color, indicator) = match result {
                    Ok(duration) => (
                        "\x1b[1;32m",
                        Cow::Owned(format!("{:3}ms", duration.as_millis())),
                    ),
                    Err(_) => ("\x1b[1;31m", Cow::Borrowed("    ●")),
                };
                command_line.print_literal(&format!(
                    "{}{}\x1b[0m {:label_width$}\t{}\t{:host_width$}\n",
                    color,
                    indicator,
                    device.label(),
                    device.mac_address(),
                    device.host()
                ));
            }
            command_line.set_exit_status(glib::ExitCode::SUCCESS.value());
            command_line.done();
            drop(guard);
        }
    ));
    glib::ExitCode::SUCCESS
}

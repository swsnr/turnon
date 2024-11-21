// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use gio::prelude::*;
use glib::dpgettext2;
use gtk::gio;

use crate::{
    config::G_LOG_DOMAIN,
    model::{Device, Devices},
};

async fn turn_on_device(command_line: &gio::ApplicationCommandLine, device: &Device) {
    match device.wol().await {
        Ok(_) => {
            command_line.set_exit_status(glib::ExitCode::SUCCESS.value());
            command_line.print_literal(
                &dpgettext2(
                    None,
                    "option.turn-on-device.message",
                    "Sent magic packet to mac address %1 of device %2\n",
                )
                .replace("%1", &device.mac_addr6().to_string())
                .replace("%2", &device.label()),
            );
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
            command_line.set_exit_status(glib::ExitCode::FAILURE.value());
        }
    }
    command_line.done();
}

pub fn turn_on_device_by_label(
    command_line: &gio::ApplicationCommandLine,
    model: &Devices,
    label: String,
) -> glib::ExitCode {
    glib::debug!("Turning on device in response to command line argument");
    match model.into_iter().find(|d| d.label() == label) {
        Some(device) => {
            glib::spawn_future_local(glib::clone!(
                #[strong]
                command_line,
                async move { turn_on_device(&command_line, &device).await }
            ));
            glib::ExitCode::SUCCESS
        }
        None => {
            command_line.printerr_literal(
                &dpgettext2(
                    None,
                    "option.turn-on-device.error",
                    "No device found for label %s\n",
                )
                .replace("%s", &label),
            );
            glib::ExitCode::FAILURE
        }
    }
}

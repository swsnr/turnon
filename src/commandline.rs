// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use gio::prelude::*;
use glib::dpgettext2;
use gtk::gio;

use crate::{app::TurnOnApplication, config::G_LOG_DOMAIN, model::Device};

async fn turn_on_device(
    command_line: &gio::ApplicationCommandLine,
    device: &Device,
) -> glib::ExitCode {
    match device.wol().await {
        Ok(_) => {
            command_line.print_literal(
                &dpgettext2(
                    None,
                    "option.turn-on-device.message",
                    "Sent magic packet to mac address %1 of device %2\n",
                )
                .replace("%1", &device.mac_addr6().to_string())
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
    label: String,
) -> glib::ExitCode {
    let guard = app.hold();
    glib::debug!("Turning on device in response to command line argument");
    match app.model().into_iter().find(|d| d.label() == label) {
        Some(device) => {
            glib::spawn_future_local(glib::clone!(
                #[strong]
                command_line,
                async move {
                    let exit_code = turn_on_device(&command_line, &device).await;
                    command_line.set_exit_status(exit_code.value());
                    command_line.done();
                    std::mem::drop(guard);
                }
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

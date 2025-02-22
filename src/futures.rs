// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::future::Future;

use gtk::gio::IOErrorEnum;

/// Like [`glib::future_with_timeout`] but flattens errors of fallible futures.
pub async fn future_with_timeout<T>(
    timeout: std::time::Duration,
    fut: impl Future<Output = Result<T, glib::Error>>,
) -> Result<T, glib::Error> {
    glib::future_with_timeout(timeout, fut)
        .await
        .map_err(|_| {
            glib::Error::new(
                IOErrorEnum::TimedOut,
                &format!("Timeout after {}ms", timeout.as_millis()),
            )
        })
        .and_then(|inner| inner)
}

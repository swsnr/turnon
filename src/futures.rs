// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

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

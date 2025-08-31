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
        .flatten()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use gnome_app_utils::futures::future;
    use gtk::gio::IOErrorEnum;

    use crate::testutil::block_on_new_main_context;

    #[test]
    fn future_with_timeout() {
        let result = block_on_new_main_context(super::future_with_timeout(
            Duration::new(1, 500_000_000),
            async {
                future::pending::<()>().await;
                Ok(1)
            },
        ));
        let error = result.unwrap_err();
        assert_eq!(error.kind(), Some(IOErrorEnum::TimedOut));
        assert_eq!(error.message(), "Timeout after 1500ms");
    }
}

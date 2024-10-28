// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use glib::VariantTy;
use gtk::gio::DBusMethodInvocation;

pub type InvocationResult = Result<Option<glib::Variant>, glib::Error>;

/// Extensions for DBus method invocations.
///
/// Being upstreamed at <https://github.com/gtk-rs/gtk-rs-core/pull/1558>
pub trait DBusMethodInvocationExt {
    fn return_result(self, result: InvocationResult);

    fn return_future_local<F>(self, f: F) -> glib::JoinHandle<()>
    where
        F: std::future::Future<Output = InvocationResult> + 'static;
}

impl DBusMethodInvocationExt for DBusMethodInvocation {
    fn return_result(self, result: Result<Option<glib::Variant>, glib::Error>) {
        match result {
            Ok(Some(value)) if !value.is_type(VariantTy::TUPLE) => {
                let tupled = glib::Variant::tuple_from_iter(std::iter::once(value));
                self.return_value(Some(&tupled));
            }
            Ok(value) => self.return_value(value.as_ref()),
            Err(error) => self.return_gerror(error),
        }
    }

    fn return_future_local<F>(self, f: F) -> glib::JoinHandle<()>
    where
        F: std::future::Future<Output = InvocationResult> + 'static,
    {
        glib::spawn_future_local(async move {
            self.return_result(f.await);
        })
    }
}

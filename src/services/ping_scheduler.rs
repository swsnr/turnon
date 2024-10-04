// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::time::Duration;

use glib::object::ObjectExt;
use glib::subclass::prelude::*;

glib::wrapper! {
    pub struct PingScheduler(ObjectSubclass<imp::PingScheduler>);
}

const PING_INTERVAL: Duration = Duration::from_secs(5);

impl PingScheduler {
    /// Start scheduling pings at a fixed interval.
    pub fn start(&self) {
        if self.imp().timer.borrow().is_none() {
            // Schedule pings, and then trigger an initial ping.
            let id = glib::timeout_add_local(
                PING_INTERVAL,
                glib::clone!(
                    #[weak(rename_to=scheduler)]
                    &self,
                    #[upgrade_or]
                    glib::ControlFlow::Break,
                    move || {
                        scheduler.emit_by_name::<()>("ping", &[]);
                        glib::ControlFlow::Continue
                    }
                ),
            );
            self.imp().timer.replace(Some(id));
            self.emit_by_name::<()>("ping", &[]);
        }
    }

    /// Stop scheduling pings.
    pub fn stop(&self) {
        self.imp().stop();
    }

    pub fn connect_ping<F>(&self, callback: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self) + 'static,
    {
        self.connect_local(
            "ping",
            false,
            glib::clone!(
                #[weak(rename_to=scheduler)]
                &self,
                #[upgrade_or_default]
                move |_| {
                    callback(&scheduler);
                    None
                }
            ),
        )
    }
}

impl Default for PingScheduler {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

mod imp {
    use std::cell::RefCell;
    use std::sync::LazyLock;

    use glib::subclass::{prelude::*, Signal};
    use glib::SourceId;

    #[derive(Default)]
    pub struct PingScheduler {
        /// The ongoing timer if any.
        pub timer: RefCell<Option<SourceId>>,
    }

    impl PingScheduler {
        pub fn stop(&self) {
            if let Some(old_timer) = self.timer.replace(None) {
                old_timer.remove();
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PingScheduler {
        const NAME: &'static str = "PingScheduler";

        type Type = super::PingScheduler;

        type ParentType = glib::Object;
    }

    impl ObjectImpl for PingScheduler {
        fn signals() -> &'static [Signal] {
            static SIGNALS: LazyLock<Vec<Signal>> =
                LazyLock::new(|| vec![Signal::builder("ping").action().build()]);
            SIGNALS.as_ref()
        }
    }

    impl Drop for PingScheduler {
        fn drop(&mut self) {
            self.stop();
        }
    }
}

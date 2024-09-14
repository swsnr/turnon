// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use gtk::gio;
use gtk::glib;

glib::wrapper! {
    pub struct WakeUpApplicationWindow(ObjectSubclass<imp::WakeUpApplicationWindow>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap,
            gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget,
            gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl WakeUpApplicationWindow {
    /// Create a new application window for the given `application`.
    pub fn new(application: &adw::Application) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }
}

mod imp {
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gtk::glib::subclass::InitializingObject;
    use gtk::{glib, CompositeTemplate};

    use crate::widgets::AddDeviceDialog;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/de/swsnr/wakeup/ui/wakeup-application-window.ui")]
    pub struct WakeUpApplicationWindow {}

    #[glib::object_subclass]
    impl ObjectSubclass for WakeUpApplicationWindow {
        const NAME: &'static str = "WakeUpApplicationWindow";

        type Type = super::WakeUpApplicationWindow;

        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("win.add_device", None, |window, _, _| {
                let dialog = AddDeviceDialog::new();
                dialog.present(Some(window));
            });
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for WakeUpApplicationWindow {}

    impl WidgetImpl for WakeUpApplicationWindow {}

    impl WindowImpl for WakeUpApplicationWindow {}

    impl ApplicationWindowImpl for WakeUpApplicationWindow {}

    impl AdwApplicationWindowImpl for WakeUpApplicationWindow {}
}

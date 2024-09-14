// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use gtk::glib;

glib::wrapper! {
    pub struct AddDeviceDialog(ObjectSubclass<imp::AddDeviceDialog>)
        @extends adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl AddDeviceDialog {
    /// Create a new dialog to add a device.
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
}

mod imp {

    use adw::subclass::prelude::*;
    use gtk::glib;
    use gtk::glib::subclass::InitializingObject;
    use gtk::CompositeTemplate;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/de/swsnr/wakeup/ui/add-device-dialog.ui")]
    pub struct AddDeviceDialog {}

    #[glib::object_subclass]
    impl ObjectSubclass for AddDeviceDialog {
        const NAME: &'static str = "AddDeviceDialog";

        type Type = super::AddDeviceDialog;

        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AddDeviceDialog {}

    impl WidgetImpl for AddDeviceDialog {}

    impl AdwDialogImpl for AddDeviceDialog {}
}

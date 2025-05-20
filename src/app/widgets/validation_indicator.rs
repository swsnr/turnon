// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

use gtk::glib;

glib::wrapper! {
    pub struct ValidationIndicator(ObjectSubclass<imp::ValidationIndicator>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ValidationIndicator {
    /// Create a new dialog to add a device.
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
}

impl Default for ValidationIndicator {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use std::cell::{Cell, RefCell};

    use adw::subclass::prelude::*;
    use gtk::CompositeTemplate;
    use gtk::glib;
    use gtk::glib::Properties;
    use gtk::glib::prelude::*;
    use gtk::glib::subclass::InitializingObject;

    #[derive(CompositeTemplate, Default, Properties)]
    #[template(resource = "/de/swsnr/turnon/ui/validation-indicator.ui")]
    #[properties(wrapper_type = super::ValidationIndicator)]
    pub struct ValidationIndicator {
        #[property(get, set)]
        is_valid: Cell<bool>,
        #[property(get, set)]
        feedback: RefCell<String>,
        #[template_child]
        indicator: TemplateChild<gtk::Stack>,
        #[template_child]
        invalid: TemplateChild<gtk::Widget>,
        #[template_child]
        valid: TemplateChild<gtk::Widget>,
    }

    impl ValidationIndicator {
        fn update(&self) {
            let child = if self.is_valid.get() {
                self.valid.get()
            } else {
                self.invalid.get()
            };
            self.indicator.set_visible_child(&child);
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ValidationIndicator {
        const NAME: &'static str = "ValidationIndicator";

        type Type = super::ValidationIndicator;

        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ValidationIndicator {
        fn constructed(&self) {
            self.parent_constructed();
            self.update();
            self.obj().connect_is_valid_notify(|o| {
                o.imp().update();
            });
        }
    }

    impl WidgetImpl for ValidationIndicator {}

    impl BinImpl for ValidationIndicator {}
}

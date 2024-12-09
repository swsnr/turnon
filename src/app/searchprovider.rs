// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Utilities for the search provider of Turn On.

use glib::{dpgettext2, ControlFlow, Variant, VariantDict};
use gtk::gio::{
    DBusMethodInvocation, ListStore, Notification, NotificationPriority, RegistrationId,
};
use gtk::prelude::*;

use crate::app::TurnOnApplication;
use crate::config::G_LOG_DOMAIN;
use crate::dbus::invocation::DBusMethodInvocationExt;
use crate::dbus::searchprovider2::{self, ActivateResult, GetResultMetas, MethodCall};

use super::model::Device;

fn matches_terms<S: AsRef<str>>(device: &Device, terms: &[S]) -> bool {
    let label = device.label().to_lowercase();
    let host = device.host().to_lowercase();
    terms.iter().all(|term| {
        let term = term.as_ref().to_lowercase();
        label.contains(&term) || host.contains(&term)
    })
}

/// Get a result set.
///
/// Return the id of all devices which match all of `terms`, either in their
/// label or their host.
///
/// The ID of a device is simply the stringified position in the list of devices.
fn get_ids_for_terms<S: AsRef<str>>(devices: &ListStore, terms: &[S]) -> Vec<String> {
    devices
        .into_iter()
        .map(|obj| obj.unwrap().downcast::<Device>().unwrap())
        // Enumerate first so that the index is correct
        .enumerate()
        .filter(|(_, device)| matches_terms(device, terms))
        .map(|(i, _)| i.to_string())
        .collect::<Vec<_>>()
}

fn get_result_set<S: AsRef<str>>(app: &TurnOnApplication, terms: &[S]) -> Variant {
    let results = get_ids_for_terms(app.model(), terms);
    (results,).into()
}

async fn activate_result(
    app: &TurnOnApplication,
    call: ActivateResult,
) -> Result<Option<Variant>, glib::Error> {
    let device = call
        .identifier
        .parse::<u32>()
        .ok()
        .and_then(|n| app.model().item(n))
        .map(|o| o.downcast::<Device>().unwrap());
    glib::trace!(
        "Activating device at index {}, device found? {}",
        call.identifier,
        device.is_some()
    );
    match device {
        None => {
            glib::warn!("Failed to find device with id {}", call.identifier);
            Ok(None)
        }
        Some(device) => device
            .wol()
            .await
            .inspect_err(|_| {
                let notification = Notification::new(&dpgettext2(
                    None,
                    "search-provider.notification.title",
                    "Failed to send magic packet",
                ));
                notification.set_body(Some(
                    &dpgettext2(
                        None,
                        "search-provider.notification.body",
                        "Failed to send magic packet to mac address %1 of device %2.",
                    )
                    .replace("%1", &device.mac_addr6().to_string())
                    .replace("%2", &device.label()),
                ));
                notification.set_priority(NotificationPriority::Urgent);
                app.send_notification(None, &notification);
            })
            .inspect(|_| {
                let notification = Notification::new(&dpgettext2(
                    None,
                    "search-provider.notification.title",
                    "Sent magic packet",
                ));
                notification.set_body(Some(
                    &dpgettext2(
                        None,
                        "search-provider.notification.body",
                        "Sent magic packet to mac address %1 of device %2.",
                    )
                    .replace("%1", &device.mac_addr6().to_string())
                    .replace("%2", &device.label()),
                ));
                let id = glib::uuid_string_random();
                app.send_notification(Some(&id), &notification);
                glib::timeout_add_seconds_local(
                    10,
                    glib::clone!(
                        #[weak]
                        app,
                        #[upgrade_or]
                        ControlFlow::Break,
                        move || {
                            app.withdraw_notification(&id);
                            ControlFlow::Break
                        }
                    ),
                );
            })
            .map(|_| None),
    }
}

fn get_result_metas(app: &TurnOnApplication, call: GetResultMetas) -> Option<Variant> {
    let metas: Vec<VariantDict> = call
        .identifiers
        .iter()
        .filter_map(|id| {
            id.parse::<u32>()
                .ok()
                .and_then(|n| app.model().item(n))
                .map(|obj| {
                    let device = obj.downcast::<Device>().unwrap();
                    let metas = VariantDict::new(None);
                    metas.insert("id", id);
                    metas.insert("name", device.label());
                    metas.insert("description", device.host());
                    metas
                })
        })
        .collect::<Vec<_>>();
    Some((metas,).into())
}

async fn dispatch_method_call(
    app: TurnOnApplication,
    call: MethodCall,
) -> Result<Option<Variant>, glib::Error> {
    use MethodCall::*;
    match call {
        GetInitialResultSet(c) => {
            glib::trace!("Initial search for terms {:?}", c.terms);
            Ok(Some(get_result_set(&app, c.terms.as_slice())))
        }
        GetSubsearchResultSet(c) => {
            glib::trace!(
                "Sub-search for terms {:?}, with initial results {:?}",
                c.terms,
                c.previous_results
            );
            // We just search fresh again, since our model is neither that big nor that complicated
            Ok(Some(get_result_set(&app, c.terms.as_slice())))
        }
        GetResultMetas(c) => Ok(get_result_metas(&app, c)),
        ActivateResult(c) => activate_result(&app, c).await,
        LaunchSearch(c) => {
            glib::debug!("Launching search for terms {:?}", &c.terms);
            // We don't have in-app search (yet?) so let's just raise our main window
            app.activate();
            Ok(None)
        }
    }
}

fn handle_search_provider_method_call(
    app: TurnOnApplication,
    method_name: &str,
    parameters: Variant,
    invocation: DBusMethodInvocation,
) {
    let call = searchprovider2::MethodCall::parse(method_name, parameters);
    invocation.return_future_local(async move { dispatch_method_call(app, call?).await });
}

/// Register the Turn On search provider for `app`.
///
/// Register a search provider for devices on the DBus connection of `app`.
/// The search provider exposes devices from the `app` model to GNOME Shell,
/// and allows to turn on devices directly from the shell overview.
pub fn register_app_search_provider(app: TurnOnApplication) -> Option<RegistrationId> {
    if let Some(connection) = app.dbus_connection() {
        let interface_info = searchprovider2::interface();
        let registration_id = connection
            .register_object("/de/swsnr/turnon/search", &interface_info)
            .method_call(glib::clone!(
                #[strong]
                app,
                move |_, sender, object_path, interface_name, method_name, parameters, invocation| {
                    glib::debug!("Sender {sender} called method {method_name} of {interface_name} on object {object_path}");
                    assert!(interface_name == searchprovider2::INTERFACE_NAME);
                    handle_search_provider_method_call(
                        app.clone(),
                        method_name,
                        parameters,
                        invocation,
                    );
                }
            ))
            .build()
            .unwrap();
        Some(registration_id)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use macaddr::MacAddr6;

    use crate::app::model::Device;

    use super::*;

    #[test]
    fn device_matches_terms_case_insensitive() {
        let device = Device::new("Server", MacAddr6::nil(), "foo.example.com");
        assert!(matches_terms(&device, &["server"]));
        assert!(matches_terms(&device, &["SERVER"]));
        assert!(matches_terms(&device, &["SeRvEr"]));
        assert!(matches_terms(&device, &["FOO"]));
        assert!(matches_terms(&device, &["fOo"]));
    }

    #[test]
    fn device_matches_terms_in_label_and_host() {
        let device = Device::new("Server", MacAddr6::nil(), "foo.example.com");
        assert!(matches_terms(&device, &["Server", "foo"]));
    }

    #[test]
    fn device_matches_terms_ignores_mac_address() {
        let device = Device::new(
            "Server",
            "a2:35:e4:9e:b4:c3".parse().unwrap(),
            "foo.example.com",
        );
        assert!(!matches_terms(&device, &["a2:35"]));
    }
}
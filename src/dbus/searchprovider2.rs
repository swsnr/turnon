// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! The search provider dbus interface.

use glib::Variant;
use gtk::gio::{DBusInterfaceInfo, DBusNodeInfo, IOErrorEnum};

/// The literal XML definition of the interface.
static XML: &str = include_str!("../../dbus-1/org.gnome.ShellSearchProvider2.xml");

/// The name of the interface.
pub static INTERFACE_NAME: &str = "org.gnome.Shell.SearchProvider2";

/// Get the DBus interface info for the search provider interface.
pub fn interface() -> DBusInterfaceInfo {
    // We unwrap here since we know that the XML is valid and contains the
    // desired interface, so none of this can realistically fail.
    DBusNodeInfo::for_xml(XML)
        .unwrap()
        .lookup_interface(INTERFACE_NAME)
        .unwrap()
}

#[derive(Debug, Variant)]
pub struct GetInitialResultSet {
    pub terms: Vec<String>,
}

#[derive(Debug, Variant)]
pub struct GetSubsearchResultSet {
    pub previous_results: Vec<String>,
    pub terms: Vec<String>,
}

#[derive(Debug, Variant)]
pub struct GetResultMetas {
    pub identifiers: Vec<String>,
}

#[derive(Debug, Variant)]
pub struct ActivateResult {
    pub identifier: String,
    pub terms: Vec<String>,
    pub timestamp: u32,
}

#[derive(Debug, Variant)]
pub struct LaunchSearch {
    pub terms: Vec<String>,
    pub timestamp: u32,
}

/// Method calls a search provider supports.
#[derive(Debug)]
pub enum MethodCall {
    GetInitialResultSet(GetInitialResultSet),
    GetSubsearchResultSet(GetSubsearchResultSet),
    GetResultMetas(GetResultMetas),
    ActivateResult(ActivateResult),
    LaunchSearch(LaunchSearch),
}

fn invalid_parameters() -> glib::Error {
    glib::Error::new(
        IOErrorEnum::InvalidArgument,
        "Invalid parameters for method",
    )
}

impl MethodCall {
    /// Parse a method call to a search provider.
    pub fn parse(method_name: &str, parameters: Variant) -> Result<MethodCall, glib::Error> {
        match method_name {
            "GetInitialResultSet" => parameters
                .get::<GetInitialResultSet>()
                .map(MethodCall::GetInitialResultSet)
                .ok_or_else(invalid_parameters),
            "GetSubsearchResultSet" => parameters
                .get::<GetSubsearchResultSet>()
                .map(MethodCall::GetSubsearchResultSet)
                .ok_or_else(invalid_parameters),
            "GetResultMetas" => parameters
                .get::<GetResultMetas>()
                .map(MethodCall::GetResultMetas)
                .ok_or_else(invalid_parameters),
            "ActivateResult" => parameters
                .get::<ActivateResult>()
                .map(MethodCall::ActivateResult)
                .ok_or_else(invalid_parameters),
            "LaunchSearch" => parameters
                .get::<LaunchSearch>()
                .map(MethodCall::LaunchSearch)
                .ok_or_else(invalid_parameters),
            _ => Err(glib::Error::new(
                IOErrorEnum::InvalidArgument,
                "Unexpected method",
            )),
        }
    }
}

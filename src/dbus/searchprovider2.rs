// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

//! The search provider dbus interface.

use glib::Variant;
use gtk::{
    gio::{DBusError, DBusInterfaceInfo, DBusNodeInfo},
    prelude::DBusMethodCall,
};

/// The literal XML definition of the interface.
const XML: &str = include_str!("../../dbus-1/org.gnome.ShellSearchProvider2.xml");

/// The name of the interface.
pub const INTERFACE_NAME: &str = "org.gnome.Shell.SearchProvider2";

/// Get the D-Bus interface info for the search provider interface.
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
    glib::Error::new(DBusError::InvalidArgs, "Invalid parameters for method")
}

impl DBusMethodCall for MethodCall {
    fn parse_call(
        _obj_path: &str,
        interface: Option<&str>,
        method: &str,
        params: glib::Variant,
    ) -> Result<Self, glib::Error> {
        if interface != Some(INTERFACE_NAME) {
            return Err(glib::Error::new(
                DBusError::UnknownInterface,
                "Unexpected interface",
            ));
        }
        match method {
            "GetInitialResultSet" => params
                .get::<GetInitialResultSet>()
                .map(MethodCall::GetInitialResultSet)
                .ok_or_else(invalid_parameters),
            "GetSubsearchResultSet" => params
                .get::<GetSubsearchResultSet>()
                .map(MethodCall::GetSubsearchResultSet)
                .ok_or_else(invalid_parameters),
            "GetResultMetas" => params
                .get::<GetResultMetas>()
                .map(MethodCall::GetResultMetas)
                .ok_or_else(invalid_parameters),
            "ActivateResult" => params
                .get::<ActivateResult>()
                .map(MethodCall::ActivateResult)
                .ok_or_else(invalid_parameters),
            "LaunchSearch" => params
                .get::<LaunchSearch>()
                .map(MethodCall::LaunchSearch)
                .ok_or_else(invalid_parameters),
            _ => Err(glib::Error::new(
                DBusError::UnknownMethod,
                "Unexpected method",
            )),
        }
    }
}

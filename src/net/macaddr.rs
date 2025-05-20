// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

//! Simple MAC address type on top of [`glib::Bytes`].
//!
//! While this is not the most efficient approach it allows storing the MAC
//! address as a glib property.

use std::fmt::Display;
use std::ops::Deref;
use std::str::FromStr;

use macaddr::MacAddr6;

/// Boxed [`MacAddr6`].
///
/// Define a MAC address type for glib, by boxing a [`MacAdd6`].
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, glib::Boxed)]
#[boxed_type(name = "MacAdd6")]
pub struct MacAddr6Boxed(MacAddr6);

impl From<MacAddr6> for MacAddr6Boxed {
    fn from(value: MacAddr6) -> Self {
        Self(value)
    }
}

impl FromStr for MacAddr6Boxed {
    type Err = macaddr::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        MacAddr6::from_str(s).map(Into::into)
    }
}

impl Deref for MacAddr6Boxed {
    type Target = MacAddr6;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for MacAddr6Boxed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

//! Boxed networking type for use as GLib property.

use std::fmt::Display;
use std::net::{AddrParseError, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::ops::Deref;
use std::str::FromStr;

use macaddr::MacAddr6;

/// Boxed [`MacAddr6`].
///
/// Define a MAC address type for GLib, by boxing a [`MacAdd6`].
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, glib::Boxed)]
#[boxed_type(name = "TurnOnMacAdd6")]
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

/// Boxed [`SocketAddrV4`].
///
/// Define a IPv4 socket address type for GLib, by boxing a standard Rust
/// [`SocketAddrV4`].
///
/// We use a boxed socket address instead of Gio's `InetSocketAddress` because
/// Rust has a much nicer API.
#[derive(Debug, Copy, Clone, Eq, PartialEq, glib::Boxed)]
#[boxed_type(name = "TurnOnSocketAddr")]
pub struct SocketAddrBoxed(SocketAddr);

impl Default for SocketAddrBoxed {
    /// The unspecified IPv4 address and port 0.
    fn default() -> Self {
        Self(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0).into())
    }
}

impl From<SocketAddr> for SocketAddrBoxed {
    fn from(value: SocketAddr) -> Self {
        Self(value)
    }
}

impl From<SocketAddrV4> for SocketAddrBoxed {
    fn from(value: SocketAddrV4) -> Self {
        SocketAddr::from(value).into()
    }
}

impl FromStr for SocketAddrBoxed {
    type Err = AddrParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        SocketAddr::from_str(s).map(Into::into)
    }
}

impl Deref for SocketAddrBoxed {
    type Target = SocketAddr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for SocketAddrBoxed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

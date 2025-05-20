// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

//! Networking for Turn On.
//!
//! This module provides various utilities around networking required by Turn On.
//! Specifically, it has a user-space ping implementation, a Wake-On-Lan
//! implementation, some helper types, and various tools for network scanning.

pub mod arpcache;
mod http;
mod macaddr;
mod monitor;
mod ping;
mod wol;

pub use http::probe_http;
pub use macaddr::MacAddr6Boxed;
pub use monitor::monitor;
pub use ping::{PingDestination, ping_address};
pub use wol::wol;

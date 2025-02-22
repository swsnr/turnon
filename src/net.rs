// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Networking for Turn On.
//!
//! This module provides various utilities around networking required by Turn On.
//! Specifically, it has a user-space ping implementation, a Wake-On-Lan
//! implementation, some helper types, and various tools for network scanning.

pub mod arpcache;
mod macaddr;
mod monitor;
mod ping;
mod wol;

pub use macaddr::MacAddr6Boxed;
pub use monitor::monitor;
pub use ping::{PingDestination, ping_address};
pub use wol::wol;

// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Networking for TurnOn.
//!
//! Contains a dead simple and somewhat inefficient ping implementation.

mod macaddr;
mod monitor;
mod ping;
mod wol;

pub use macaddr::MacAddr6Boxed;
pub use monitor::monitor;
pub use ping::{ping_address_with_timeout, PingDestination};
pub use wol::wol;

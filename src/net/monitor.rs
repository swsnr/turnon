// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Monitor network targets with periodic pings.

use std::net::IpAddr;
use std::rc::Rc;
use std::{cell::RefCell, time::Duration};

use futures_util::{Stream, StreamExt};

use crate::config::G_LOG_DOMAIN;

use super::{ping_address_with_timeout, Target};

/// Monitor a `target` at the given `interval`.
///
/// Return a stream which yields `Ok` if the target could be resolved and reply to echo requests,
/// or `Err` if a ping failed.
pub fn monitor(
    target: Target,
    interval: Duration,
) -> impl Stream<Item = Result<Duration, glib::Error>> {
    let cached_ip_address: Rc<RefCell<Option<IpAddr>>> = Default::default();
    let timeout = interval / 2;
    futures_util::stream::iter(vec![()])
        .chain(glib::interval_stream(interval))
        .enumerate()
        .map(|(seqnr, _)| (seqnr % (u16::MAX as usize)) as u16)
        .scan(cached_ip_address, move |state, seqnr| {
            let target = target.clone();
            let state = state.clone();
            async move {
                // Take any cached IP address out of the state, leaving an empty state.
                // If we get a reply from the IP address we'll cache it again after pinging it.
                let result = match state.take() {
                    // If we have a cached IP address, ping it, and cache it again
                    // if it's still reachable.
                    Some(address) => ping_address_with_timeout(address, seqnr, timeout)
                        .await
                        .inspect(|duration| {
                            glib::trace!(
                                "Cached address {address} replied to ping after {}ms and is still reachable, caching again",
                                duration.as_millis()
                            );
                            state.replace(Some(address));
                        }),
                    // If we have no cached IP address resolve the target and ping all
                    // addresses it resolves to, then cache the first reachable address.
                    None => target.ping_with_timeout(seqnr, timeout)
                        .await
                        .inspect(|(address, duration)| {
                            glib::trace!("{address} of {target} replied after {}ms, caching", duration.as_millis());
                            state.replace(Some(*address));
                        })
                        .map(|(_, duration)| duration),
                };
                Some(result)
            }
        })
}

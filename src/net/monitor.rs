// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

//! Monitor network destinations with periodic pings.

use std::net::IpAddr;
use std::rc::Rc;
use std::{cell::RefCell, time::Duration};

use futures_util::{Stream, StreamExt};

use crate::config::G_LOG_DOMAIN;
use crate::futures::future_with_timeout;

use super::{PingDestination, ping_address};

/// Monitor a network destination.
///
/// Periodically ping`destination` at the given `interval` and yield the results.
///
/// Return a stream which yields `Ok` if the destination could be resolved and
/// replied to echo requests, or `Err` if a ping failed.  In the former case,
/// return the resolved IP address and the roundtrip duration for the ping.
pub fn monitor(
    destination: PingDestination,
    interval: Duration,
) -> impl Stream<Item = Result<(IpAddr, Duration), glib::Error>> {
    let cached_ip_address: Rc<RefCell<Option<IpAddr>>> = Rc::default();
    let timeout = interval / 2;
    futures_util::stream::iter(vec![()])
        .chain(glib::interval_stream(interval))
        .enumerate()
        .map(|(seqnr, ())| u16::try_from(seqnr % usize::from(u16::MAX)).unwrap())
        .scan(cached_ip_address, move |state, seqnr| {
            let destination = destination.clone();
            let state = state.clone();
            async move {
                // Take any cached IP address out of the state, leaving an empty state.
                // If we get a reply from the IP address we'll cache it again after pinging it.
                let result = match state.take() {
                    // If we have a cached IP address, ping it, and cache it again
                    // if it's still reachable.
                    Some(address) => future_with_timeout(timeout, ping_address(address, seqnr))
                        .await
                        .inspect(|duration| {
                            glib::trace!(
                                "Cached address {address} replied to ping after \
{}ms and is still reachable, caching again",
                                duration.as_millis()
                            );
                            state.replace(Some(address));
                        })
                        .map(|duration| (address, duration)),
                    // If we have no cached IP address resolve the destination and ping all
                    // addresses it resolves to, then cache the first reachable address.
                    None => future_with_timeout(timeout, destination.ping(seqnr))
                        .await
                        .inspect(|(address, duration)| {
                            glib::trace!(
                                "{address} of {destination} replied after {}ms, caching",
                                duration.as_millis()
                            );
                            state.replace(Some(*address));
                        }),
                };
                Some(result)
            }
        })
}

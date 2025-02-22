// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! A simple user-space ping implementation.

use std::fmt::Display;
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::time::{Duration, Instant};

use futures_util::stream::FuturesUnordered;
use futures_util::{FutureExt, StreamExt, future};
use glib::IOCondition;
use gtk::gio::Cancellable;
use gtk::gio::prelude::{ResolverExt, SocketExtManual};
use gtk::gio::{self, IOErrorEnum};
use gtk::prelude::SocketExt;

use crate::config::G_LOG_DOMAIN;

#[allow(
    clippy::needless_pass_by_value,
    reason = "Taking error by value is more ergonomic with map_err"
)]
fn to_glib_error(error: std::io::Error) -> glib::Error {
    let io_error = error
        .raw_os_error()
        .map_or(IOErrorEnum::Failed, gio::io_error_from_errno);
    glib::Error::new(io_error, &error.to_string())
}

fn icmp_socket_for_address(address: IpAddr) -> Result<gio::Socket, glib::Error> {
    let (domain, proto) = match address {
        IpAddr::V4(_) => (libc::AF_INET, libc::IPPROTO_ICMP),
        IpAddr::V6(_) => (libc::AF_INET6, libc::IPPROTO_ICMPV6),
    };
    // SAFETY: We only pass integer constants here, and check for error return immediately.
    let socket = unsafe { libc::socket(domain, libc::SOCK_DGRAM, proto) };
    if socket < 0 {
        Err(to_glib_error(std::io::Error::last_os_error()))
    } else {
        // SAFETY: socket returns a new FD on success which the caller now owns.
        let socket = unsafe { OwnedFd::from_raw_fd(socket) };
        // SAFETY: from_fd has unfortunate ownership semantics: It claims the fd on
        // success, but on error the caller retains ownership of the fd.  Hence, we
        // do _not_ move out of `fd` here, but instead pass the raw fd.  In case of
        // error Rust will then just drop our owned fd as usual.  In case of success
        // the fd now belongs to the GIO socket, so we explicitly forget the
        // borrowed fd.
        let gio_socket = unsafe { gio::Socket::from_fd(socket.as_raw_fd()) }?;
        // Do not drop our fd because it is now owned by gio_socket.
        std::mem::forget(socket);
        // Make the socket non-blocking and add a reasonable timeout.
        // set_timeout takes a timeout in seconds; we go through a Duration value
        // to make this explicit.
        gio_socket.set_blocking(false);
        gio_socket.set_timeout(u32::try_from(Duration::from_secs(10).as_secs()).unwrap());
        Ok(gio_socket)
    }
}

/// Send a single ping to `ip_address`.
///
/// Return an error if pinging `ip_address` failed, or if we received a non-reply
/// response.
///
/// Otherwise return the time between echo request and echo reply, i.e. the approximate
/// roundtrip time, module scheduling inaccuraries from the operating system and
/// the underlying async executor (e.g. the glib mainloop).
pub async fn ping_address(
    ip_address: IpAddr,
    sequence_number: u16,
) -> Result<Duration, glib::Error> {
    glib::trace!("Sending ICMP echo request to {ip_address}");
    let start = Instant::now();
    let socket = icmp_socket_for_address(ip_address)?;
    let condition = socket
        .create_source_future(IOCondition::OUT, Cancellable::NONE, glib::Priority::DEFAULT)
        .await;
    if condition != glib::IOCondition::OUT {
        return Err(glib::Error::new(
            IOErrorEnum::BrokenPipe,
            &format!("Socket for {ip_address} not ready to write"),
        ));
    }

    let condition =
        socket.create_source_future(IOCondition::IN, Cancellable::NONE, glib::Priority::DEFAULT);
    let socket_address: gio::InetSocketAddress = SocketAddr::new(ip_address, 0).into();
    // An echo reply for ICMPv4 and ICMPv6 respectively.
    let r#type = match ip_address {
        IpAddr::V4(_) => 8u8,
        IpAddr::V6(_) => 128u8,
    };
    // Our ICMP packet.  ICMPv4 and ICMPv6 have the same layout, so we can use the
    // same packet for both.
    //
    // Documentation around unprivileged ICMP is somewhat sparse in Linux land, but
    // it seems that the kernel handles the checksum and the identifier for us,
    // so we can statically assemble the packet.
    let mut echo_request = [
        r#type, // Type
        0,      // code,
        0, 0, // Checksum
        0, 0, // Identifier
        0, 0, // Sequence number
        b't', b'u', b'r', b'n', b'o', b'n', b'-', b'p', b'i', b'n', b'g', b'\n', // line 1
        b't', b'u', b'r', b'n', b'o', b'n', b'-', b'p', b'i', b'n', b'g', b'\n', // line 2
        b't', b'u', b'r', b'n', b'o', b'n', b'-', b'p', b'i', b'n', b'g', b'\n', // line 3
        b't', b'u', b'r', b'n', b'o', b'n', b'-', b'p', b'i', b'n', b'g', b'\n', // line 4
    ];
    echo_request[6..8].copy_from_slice(&sequence_number.to_be_bytes());
    let bytes_written = socket.send_to(Some(&socket_address), echo_request, Cancellable::NONE)?;
    if bytes_written != echo_request.len() {
        return Err(glib::Error::new(
            IOErrorEnum::BrokenPipe,
            &format!("Failed to write full ICMP echo request to {ip_address} to socket"),
        ));
    }
    if condition.await != glib::IOCondition::IN {
        return Err(glib::Error::new(
            IOErrorEnum::BrokenPipe,
            &format!("Socket for {ip_address} not ready to read"),
        ));
    }

    // We expect a response of the same size as the echo request: The response
    // header has the same size, and the payload is mirrored back.
    let mut response = [0; 56];
    // Sanity check in case we got the array length wrong!
    assert!(response.len() == echo_request.len());
    let (bytes_received, _) = socket.receive_from(&mut response, Cancellable::NONE)?;
    let end = Instant::now();
    if bytes_received != response.len() {
        return Err(glib::Error::new(
            IOErrorEnum::BrokenPipe,
            &format!("Failed to read full ICMP echo reply from {ip_address} from socket"),
        ));
    }

    // Check that we received an echo reply.
    let response_type = match ip_address {
        IpAddr::V4(_) => 0,
        IpAddr::V6(_) => 129,
    };
    if response[0] == response_type {
        // We will not panic here, because `response` is guaranteed to be larger than 8 (see above!)
        let received_sequence_number = u16::from_be_bytes(response[6..8].try_into().unwrap());
        if sequence_number == received_sequence_number {
            Ok(end - start)
        } else {
            Err(glib::Error::new(
                IOErrorEnum::InvalidData,
                &format!(
                    "Received out of order sequence number {received_sequence_number}, expected {sequence_number}"
                ),
            ))
        }
    } else {
        Err(glib::Error::new(
            IOErrorEnum::InvalidData,
            &format!("Received unexpected response of type {}", response[0]),
        ))
    }
}

/// A network destination which we can ping.
#[derive(Debug, Clone)]
pub enum PingDestination {
    /// A DNS name which needs to be resolved first.
    Dns(String),
    /// A resolved IP address.
    Addr(IpAddr),
}

impl PingDestination {
    /// Resolve this destination into a list of IP addresses.
    ///
    /// If this destnation is an IP address just return the IP address again.
    /// Otherwise resolve this destination using the default Gio resolver, and
    /// return all addresses the name resolves to.
    pub fn resolve(&self) -> impl Future<Output = Result<Vec<IpAddr>, glib::Error>> {
        match self {
            PingDestination::Addr(address) => future::ready(Ok(vec![*address])).right_future(),
            PingDestination::Dns(host) => {
                // The destination a DNS name so let's resolve it into a list of IP addresses.
                glib::trace!("Resolving {host} to IP address");
                gio::Resolver::default()
                    .lookup_by_name_future(host)
                    .map(move |result| match result {
                        Ok(addresses) if addresses.is_empty() => Err(glib::Error::new(
                            IOErrorEnum::NotFound,
                            "No addresses found",
                        )),
                        Ok(addresses) => Ok(addresses.into_iter().map(Into::into).collect()),
                        Err(error) => Err(error),
                    })
                    .left_future()
            }
        }
    }

    /// Ping a single destination and return the first reachable address.
    ///
    /// Resolve this destination using [`resolve`] and ping all resolved addresses
    /// at once.  Then return the first address that replied, and the approximate
    /// roundtrip time to that address.
    pub async fn ping(&self, sequence_number: u16) -> Result<(IpAddr, Duration), glib::Error> {
        let addresses = self.resolve().await.inspect_err(|error| {
            glib::trace!("Failed to resolve {self} to an IP address: {error}");
        })?;
        let mut reachable_addresses = addresses
            .into_iter()
            .map(|addr| ping_address(addr, sequence_number).map(move |result| (addr, result)))
            .collect::<FuturesUnordered<_>>()
            // Filter out all address which we can't ping or which don't reply
            .filter_map(|(ip_address, result)| match result {
                Ok(duration) => {
                    glib::trace!("{ip_address} replied to ping");
                    future::ready(Some((ip_address, duration)))
                }
                Err(error) => {
                    glib::trace!("Failed to ping {ip_address}: {error}");
                    future::ready(None)
                }
            });
        reachable_addresses.next().await.ok_or_else(|| {
            glib::Error::new(
                IOErrorEnum::NotFound,
                &format!("Target {self} had no reachable addresses"),
            )
        })
    }
}

impl From<String> for PingDestination {
    fn from(host: String) -> Self {
        host.parse().map_or_else(|_| Self::Dns(host), Self::Addr)
    }
}

impl Display for PingDestination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PingDestination::Dns(host) => host.fmt(f),
            PingDestination::Addr(ip_addr) => ip_addr.fmt(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        net::{IpAddr, Ipv4Addr, Ipv6Addr},
        str::FromStr,
        time::Duration,
    };

    use gtk::gio::IOErrorEnum;

    use crate::futures::future_with_timeout;

    use super::PingDestination;

    #[glib::async_test]
    async fn ping_loopback_ipv4() {
        let duration = super::ping_address(Ipv4Addr::LOCALHOST.into(), 4)
            .await
            .unwrap();
        // A reasonable sanity test
        assert!(duration < Duration::from_secs(5));
    }

    #[glib::async_test]

    async fn ping_loopback_ipv6() {
        let duration = super::ping_address(Ipv6Addr::LOCALHOST.into(), 4)
            .await
            .unwrap();
        // A reasonable sanity test
        assert!(duration < Duration::from_secs(5));
    }

    #[glib::async_test]

    async fn ping_with_timeout_unroutable() {
        let error = future_with_timeout(
            Duration::from_secs(1),
            super::ping_address(Ipv4Addr::from_str("192.0.2.42").unwrap().into(), 4),
        )
        .await
        .unwrap_err();
        assert!(error.matches(IOErrorEnum::TimedOut));
        assert_eq!(error.message(), "Timeout after 1000ms");
    }

    #[glib::async_test]
    async fn ping_destination_resolve() {
        assert_eq!(
            PingDestination::Addr(Ipv4Addr::LOCALHOST.into())
                .resolve()
                .await
                .unwrap(),
            vec![IpAddr::V4(Ipv4Addr::LOCALHOST)]
        );
        assert_eq!(
            PingDestination::Addr(Ipv6Addr::LOCALHOST.into())
                .resolve()
                .await
                .unwrap(),
            vec![IpAddr::V6(Ipv6Addr::LOCALHOST)]
        );
        let addresses = PingDestination::Dns("localhost".into())
            .resolve()
            .await
            .unwrap();
        assert!(addresses.len() >= 2);
        assert!(addresses.contains(&IpAddr::V4(Ipv4Addr::LOCALHOST)));
        assert!(addresses.contains(&IpAddr::V6(Ipv6Addr::LOCALHOST)));
    }
}

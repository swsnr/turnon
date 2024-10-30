// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Networking for TurnOn.
//!
//! Contains a dead simple and somewhat inefficient ping implementation.

use std::cell::RefCell;
use std::fmt::Display;
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::os::fd::{AsRawFd, OwnedFd};
use std::rc::Rc;
use std::time::Duration;

use futures_util::stream;
use futures_util::stream::FuturesUnordered;
use futures_util::{future, select_biased, FutureExt, Stream, StreamExt, TryFutureExt};
use glib::IOCondition;
use gtk::gio::prelude::{ResolverExt, SocketExt, SocketExtManual};
use gtk::gio::{self, IOErrorEnum};
use gtk::gio::{Cancellable, InetAddress};
use macaddr::MacAddr6;
use socket2::*;

use crate::config::G_LOG_DOMAIN;

fn to_glib_error(error: std::io::Error) -> glib::Error {
    let io_error = error
        .raw_os_error()
        .map_or(IOErrorEnum::Failed, gio::io_error_from_errno);
    glib::Error::new(io_error, &error.to_string())
}

fn create_dgram_socket(domain: Domain, protocol: Protocol) -> Result<gio::Socket, glib::Error> {
    let socket =
        socket2::Socket::new_raw(domain, Type::DGRAM, Some(protocol)).map_err(to_glib_error)?;
    socket.set_nonblocking(true).map_err(to_glib_error)?;
    socket
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(to_glib_error)?;
    let fd = OwnedFd::from(socket);
    // SAFETY: from_fd has unfortunate ownership semantics: It claims the fd on
    // success, but on error the caller retains ownership of the fd.  Hence, we
    // do _not_ move out of `fd` here, but instead pass the raw fd.  In case of
    // error Rust will then just drop our owned fd as usual.  In case of success
    // the fd now belongs to the GIO socket, so we explicitly forget the
    // borrowed fd.
    let gio_socket = unsafe { gio::Socket::from_fd(fd.as_raw_fd()) }?;
    // Do not drop our fd because it is now owned by gio_socket
    std::mem::forget(fd);
    Ok(gio_socket)
}

/// The target to ping.
#[derive(Debug, Clone)]
pub enum Target {
    /// Ping a DNS name which we need to resolve first.
    Dns(String),
    /// Ping a resolved IP address.
    Addr(IpAddr),
}

impl From<String> for Target {
    fn from(host: String) -> Self {
        host.parse().map_or_else(|_| Self::Dns(host), Self::Addr)
    }
}

impl Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Target::Dns(host) => host.fmt(f),
            Target::Addr(ip_addr) => ip_addr.fmt(f),
        }
    }
}

/// Send a single ping to `ip_address`.
///
/// Return an error if pinging `ip_address` failed, or if we received a non-reply
/// response.
async fn ping(ip_address: IpAddr, sequence_number: u16) -> Result<(), glib::Error> {
    glib::trace!("Sending ICMP echo request to {ip_address}");
    let (domain, protocol) = match ip_address {
        IpAddr::V4(_) => (Domain::IPV4, Protocol::ICMPV4),
        IpAddr::V6(_) => (Domain::IPV6, Protocol::ICMPV6),
    };
    let socket = create_dgram_socket(domain, protocol)?;
    let condition = socket
        .create_source_future(IOCondition::OUT, Cancellable::NONE, glib::Priority::DEFAULT)
        .await;
    if condition != glib::IOCondition::OUT {
        socket.close().ok();
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
        socket.close().ok();
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
    socket.close().ok();
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
            Ok(())
        } else {
            Err(glib::Error::new(
                IOErrorEnum::InvalidData,
                &format!("Received out of order sequence number {received_sequence_number}, expected {sequence_number}"),
            ))
        }
    } else {
        Err(glib::Error::new(
            IOErrorEnum::InvalidData,
            &format!("Received unexpected response of type {}", response[0]),
        ))
    }
}

fn to_rust_addresses(
    result: Result<Vec<InetAddress>, glib::Error>,
) -> Result<Vec<IpAddr>, glib::Error> {
    result.and_then(|addresses| {
        if addresses.is_empty() {
            Err(glib::Error::new(
                IOErrorEnum::NotFound,
                "No addresses found",
            ))
        } else {
            Ok(addresses.into_iter().map(Into::into).collect())
        }
    })
}

/// Monitor a `target` at the given `interval`.
///
/// Return a stream which yields `Ok` if the target could be resolved and reply to echo requests,
/// or `Err` if a ping failed.
pub fn monitor(target: Target, interval: Duration) -> impl Stream<Item = Result<(), glib::Error>> {
    let cached_ip_address: Rc<RefCell<Option<IpAddr>>> = Default::default();
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
                let addresses = match state.take() {
                    Some(address) => {
                        glib::trace!("Using cached IP address {address}");
                        future::ready(vec![address]).right_future()
                    }
                    // We don't have a cached IP address, so let's look at the target.
                    None => match target {
                        Target::Addr(address) => future::ready(vec![address]).right_future(),
                        Target::Dns(ref host) => {
                            // The target is a DNS name so let's resolve it into a list of IP addresses.
                            glib::trace!("Resolving {host} to IP address");
                            gio::Resolver::default()
                                .lookup_by_name_future(host)
                                .map(to_rust_addresses)
                                .inspect_err(|error| {
                                    glib::trace!(
                                        "Failed to resolve {target} to an IP address: {error}"
                                    );
                                })
                                .map(|addresses| addresses.unwrap_or_default())
                                .left_future()
                        }
                    },
                };
                let mut reachable_addresses = stream::once(addresses)
                    .flat_map(|addresses| {
                        addresses
                            .into_iter()
                            .map(|addr| ping(addr, seqnr).map(move |result| (addr, result)))
                            .collect::<FuturesUnordered<_>>()
                    })
                    // Filter out all address which we can't ping or which don't reply
                    .filter_map(|(ip_address, result)| match result {
                        Ok(_) => {
                            glib::trace!("{ip_address} replied to ping");
                            future::ready(Some(ip_address))
                        }
                        Err(error) => {
                            glib::trace!("Failed to ping {ip_address}: {error}");
                            future::ready(None)
                        }
                    });

                // Select the first reachable address within a timeout. We always
                // return Some here to make scan continue at the next interval.
                select_biased! {
                    reachable_address = reachable_addresses.next() => match reachable_address {
                        // The stream was empty, meaning we failed to ping any address
                        None => Some(Err(glib::Error::new(
                            IOErrorEnum::NotFound,
                            &format!("Target {target} had no reachable addresses")
                        ))),
                        Some(address) => {
                            // Cache the first reachable address we get for the next ping.
                            state.replace(Some(address));
                            Some(Ok(()))
                        },
                    },
                    _ = glib::timeout_future(interval).fuse() => Some(Err(
                        glib::Error::new(IOErrorEnum::TimedOut, &format!("No response received from {target} after {}s", interval.as_secs()))
                    )),
                }
            }
        })
}

/// Write a magic packet for the given `mac_address` to `sink`.
fn write_magic_packet<W: Write>(sink: &mut W, mac_address: MacAddr6) -> std::io::Result<()> {
    sink.write_all(&[0xff; 6])?;
    for _ in 0..16 {
        sink.write_all(mac_address.as_bytes())?;
    }
    Ok(())
}

/// Send a magic Wake On LAN packet to the given `mac_address`.
///
/// Sends the WoL package as UDP package to port 9 on the IPv4 broadcast address.
pub async fn wol(mac_address: MacAddr6) -> Result<(), glib::Error> {
    let socket = gio::Socket::new(
        gio::SocketFamily::Ipv4,
        gio::SocketType::Datagram,
        gio::SocketProtocol::Udp,
    )?;
    socket.set_broadcast(true);

    let condition = socket
        .create_source_future(IOCondition::OUT, Cancellable::NONE, glib::Priority::DEFAULT)
        .await;
    if condition != glib::IOCondition::OUT {
        socket.close().ok();
        return Err(glib::Error::new(
            IOErrorEnum::BrokenPipe,
            &format!("Socket for waking {mac_address} not ready to write"),
        ));
    }
    let mut payload = [0; 102];
    write_magic_packet(&mut payload.as_mut_slice(), mac_address).unwrap();
    let broadcast_and_discard_address: gio::InetSocketAddress =
        SocketAddr::new(Ipv4Addr::BROADCAST.into(), 9).into();
    let bytes_sent = socket.send_to(
        Some(&broadcast_and_discard_address),
        payload,
        Cancellable::NONE,
    )?;
    assert!(bytes_sent == 102);
    socket.close().ok();
    Ok(())
}

#[cfg(test)]
mod tests {
    use macaddr::MacAddr6;

    use crate::net::write_magic_packet;

    #[test]
    fn test_write_magic_packet() {
        let mac_address = "26:CE:55:A5:C2:33".parse::<MacAddr6>().unwrap();
        let mut buffer = Vec::new();
        write_magic_packet(&mut buffer, mac_address).unwrap();
        let expected_packet: [u8; 102] = [
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, // // Six all 1 bytes
            0x26, 0xCE, 0x55, 0xA5, 0xC2, 0x33, // 16 repetitions of the mac address
            0x26, 0xCE, 0x55, 0xA5, 0xC2, 0x33, //
            0x26, 0xCE, 0x55, 0xA5, 0xC2, 0x33, //
            0x26, 0xCE, 0x55, 0xA5, 0xC2, 0x33, //
            0x26, 0xCE, 0x55, 0xA5, 0xC2, 0x33, //
            0x26, 0xCE, 0x55, 0xA5, 0xC2, 0x33, //
            0x26, 0xCE, 0x55, 0xA5, 0xC2, 0x33, //
            0x26, 0xCE, 0x55, 0xA5, 0xC2, 0x33, //
            0x26, 0xCE, 0x55, 0xA5, 0xC2, 0x33, //
            0x26, 0xCE, 0x55, 0xA5, 0xC2, 0x33, //
            0x26, 0xCE, 0x55, 0xA5, 0xC2, 0x33, //
            0x26, 0xCE, 0x55, 0xA5, 0xC2, 0x33, //
            0x26, 0xCE, 0x55, 0xA5, 0xC2, 0x33, //
            0x26, 0xCE, 0x55, 0xA5, 0xC2, 0x33, //
            0x26, 0xCE, 0x55, 0xA5, 0xC2, 0x33, //
            0x26, 0xCE, 0x55, 0xA5, 0xC2, 0x33, //
        ];
        assert_eq!(buffer.as_slice(), expected_packet.as_slice());
    }
}

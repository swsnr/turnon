// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Networking for TurnOn.
//!
//! Contains a dead simple and somewhat inefficient ping implementation.

use std::cell::RefCell;
use std::error::Error;
use std::fmt::Display;
use std::io::{ErrorKind, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::os::fd::{AsRawFd, OwnedFd};
use std::rc::Rc;
use std::time::Duration;

use etherparse::{IcmpEchoHeader, Icmpv4Slice, Icmpv4Type, Icmpv6Slice, Icmpv6Type};
use futures_util::stream;
use futures_util::stream::FuturesUnordered;
use futures_util::{future, select_biased, FutureExt, Stream, StreamExt, TryFutureExt};
use glib::IOCondition;
use gtk::gio;
use gtk::gio::prelude::{ResolverExt, SocketExt, SocketExtManual};
use gtk::gio::InetAddressBytes;
use gtk::gio::{Cancellable, InetAddress};
use gtk::prelude::{InetAddressExt, InetAddressExtManual};
use macaddr::MacAddr6;
use socket2::*;

fn create_dgram_socket(domain: Domain, protocol: Protocol) -> Result<gio::Socket, Box<dyn Error>> {
    let socket = socket2::Socket::new_raw(domain, Type::DGRAM, Some(protocol))?;
    socket.set_nonblocking(true)?;
    socket.set_read_timeout(Some(Duration::from_secs(10)))?;
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

/// Convert a GIO inet address to Rust.
///
/// We deliberately do not use the From impl from gtk-rs-core, because it's
/// broken, see <https://github.com/gtk-rs/gtk-rs-core/issues/1535>.
fn to_rust(address: InetAddress) -> IpAddr {
    match address.to_bytes() {
        Some(InetAddressBytes::V4(bytes)) => IpAddr::from(*bytes),
        Some(InetAddressBytes::V6(bytes)) => IpAddr::from(*bytes),
        None => panic!("Unsupported address family: {:?}", address.family()),
    }
}

/// A reply to a single ping.
#[derive(Debug)]
pub enum PingReply {
    /// A correct ICMP echo reply.
    EchoReply,
    /// Another response for an ICMP v4 echo request.
    OtherV4(Icmpv4Type),
    /// Another response for an ICMP v6 echo request.
    OtherV6(Icmpv6Type),
}

/// Send a single ping to `ip_address`.
async fn ping(ip_address: IpAddr) -> Result<PingReply, Box<dyn Error>> {
    log::trace!("Sending ICMP echo request to {ip_address}");
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
        return Err(std::io::Error::new(
            ErrorKind::BrokenPipe,
            format!("Socket for {ip_address} not ready to write"),
        )
        .into());
    }

    let condition =
        socket.create_source_future(IOCondition::IN, Cancellable::NONE, glib::Priority::DEFAULT);
    let socket_address: gio::InetSocketAddress = SocketAddr::new(ip_address, 0).into();
    let header = IcmpEchoHeader { id: 42, seq: 23 };
    let payload = b"turnon-ping turnon-ping turnon-ping turnon-ping";
    let mut packet = match ip_address {
        IpAddr::V4(_) => {
            let echo = etherparse::Icmpv4Type::EchoRequest(header);
            let header = etherparse::Icmpv4Header::with_checksum(echo, payload);
            header.to_bytes().to_vec()
        }
        IpAddr::V6(_) => {
            let echo = etherparse::Icmpv6Type::EchoRequest(header);
            let header =
                etherparse::Icmpv6Header::with_checksum(echo, [0; 16], [0; 16], payload).unwrap();
            header.to_bytes().to_vec()
        }
    };
    packet.extend_from_slice(payload);
    let bytes_written = socket.send_to(Some(&socket_address), &packet, Cancellable::NONE)?;
    assert!(bytes_written == packet.len());
    if condition.await != glib::IOCondition::IN {
        socket.close().ok();
        return Err(std::io::Error::new(
            ErrorKind::BrokenPipe,
            format!("Socket for {ip_address} not ready to read"),
        )
        .into());
    }

    let mut buffer = [0; 128];
    let (bytes_received, _) = socket.receive_from(&mut buffer, Cancellable::NONE)?;
    socket.close().ok();
    match ip_address {
        IpAddr::V4(_) => match Icmpv4Slice::from_slice(&buffer[..bytes_received])?.icmp_type() {
            Icmpv4Type::EchoReply(_) => Ok(PingReply::EchoReply),
            other => Ok(PingReply::OtherV4(other)),
        },
        IpAddr::V6(_) => match Icmpv6Slice::from_slice(&buffer[..bytes_received])?.icmp_type() {
            Icmpv6Type::EchoReply(_) => Ok(PingReply::EchoReply),
            other => Ok(PingReply::OtherV6(other)),
        },
    }
}

fn to_rust_addresses(
    result: Result<Vec<InetAddress>, glib::Error>,
) -> Result<Vec<IpAddr>, Box<dyn Error>> {
    match result {
        Ok(addresses) if addresses.is_empty() => {
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "No addresses found").into())
        }
        Ok(addresses) => Ok(addresses.into_iter().map(to_rust).collect()),
        Err(error) => Err(error.into()),
    }
}

/// Monitor a `target` at the given `interval`.
///
/// Return a stream providing whether the target is online.
pub fn monitor(target: Target, interval: Duration) -> impl Stream<Item = bool> {
    let cached_ip_address: Rc<RefCell<Option<IpAddr>>> = Default::default();
    futures_util::stream::iter(vec![()])
        .chain(glib::interval_stream(interval))
        .scan(cached_ip_address, move |state, _| {
            let target = target.clone();
            let state = state.clone();
            async move {
                // Take any cached IP address out of the state, leaving an empty state.
                // If we get a reply from the IP address we'll cache it again after pinging it.
                let addresses = match state.take() {
                    Some(address) => {
                        log::trace!("Using cached IP address {address}");
                        future::ready(vec![address]).right_future()
                    }
                    // We don't have a cached IP address, so let's look at the target.
                    None => match target {
                        Target::Addr(address) => future::ready(vec![address]).right_future(),
                        Target::Dns(ref host) => {
                            // The target is a DNS name so let's resolve it into a list of IP addresses.
                            log::trace!("Resolving {host} to IP address");
                            gio::Resolver::default()
                                .lookup_by_name_future(host)
                                .map(to_rust_addresses)
                                .inspect_err(|error| {
                                    log::trace!(
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
                            .map(|addr| ping(addr).map(move |result| (addr, result)))
                            .collect::<FuturesUnordered<_>>()
                    })
                    // Filter out all address which we can't ping or which don't reply
                    .filter_map(|(ip_address, result)| match result {
                        Ok(PingReply::EchoReply) => {
                            log::trace!("{ip_address} replied to ping");
                            future::ready(Some(ip_address))
                        }
                        Ok(PingReply::OtherV4(other)) => {
                            log::trace!("{ip_address} did not reply: {other:?}");
                            future::ready(None)
                        }
                        Ok(PingReply::OtherV6(other)) => {
                            log::trace!("{ip_address} did not reply: {other:?}");
                            future::ready(None)
                        }
                        Err(error) => {
                            log::trace!("Failed to ping {ip_address}: {error}");
                            future::ready(None)
                        }
                    });

                // Select the first reachable address within a timeout. We always
                // return Some here to make scan continue at the next interval.
                select_biased! {
                    reachable_address = reachable_addresses.next() => match reachable_address {
                        // The stream was empty, meaning we failed to ping any address
                        None => Some(false),
                        Some(address) => {
                            // Cache the first reachable address we get for the next ping.
                            state.replace(Some(address));
                            Some(true)
                        },
                    },
                    _ = glib::timeout_future(interval).fuse() => Some(false),
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

/// Send a WakeOnLan packet to the given `mac_address`.
///
/// Sends the WoL package as UDP package to port 9 on the IPv4 broadcast address.
pub async fn wol(mac_address: MacAddr6) -> Result<(), Box<dyn Error>> {
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
        return Err(std::io::Error::new(
            ErrorKind::BrokenPipe,
            format!("Socket for waking {mac_address} not ready to write"),
        )
        .into());
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
    use std::net::IpAddr;

    use gtk::gio;
    use macaddr::MacAddr6;

    use crate::net::{to_rust, write_magic_packet};

    #[test]
    fn test_ipv6_to_rust() {
        let rust_addr = "2606:50c0:8000::153".parse::<IpAddr>().unwrap();
        assert!(rust_addr.is_ipv6());
        let gio_addr = gio::InetAddress::from(rust_addr);
        assert_eq!(rust_addr, to_rust(gio_addr));
    }

    #[test]
    fn test_ipv4_to_rust() {
        let rust_addr = "185.199.108.153".parse::<IpAddr>().unwrap();
        assert!(rust_addr.is_ipv4());
        let gio_addr = gio::InetAddress::from(rust_addr);
        assert_eq!(rust_addr, to_rust(gio_addr));
    }

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

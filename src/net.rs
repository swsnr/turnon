// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Networking for Wakeup.
//!
//! Contains a dead simple and somewhat inefficient ping implementation.

use std::cell::RefCell;
use std::error::Error;
use std::fmt::Display;
use std::net::{IpAddr, SocketAddr};
use std::os::fd::{AsRawFd, OwnedFd};
use std::rc::Rc;
use std::time::Duration;

use etherparse::{IcmpEchoHeader, Icmpv4Slice, Icmpv4Type, Icmpv6Slice, Icmpv6Type};
use futures_util::stream::FuturesUnordered;
use futures_util::{future, select_biased, FutureExt, Stream, StreamExt};
use glib::IOCondition;
use gtk::gio;
use gtk::gio::prelude::{ResolverExt, SocketExt, SocketExtManual};
use gtk::gio::InetAddressBytes;
use gtk::gio::{Cancellable, InetAddress};
use gtk::prelude::{InetAddressExt, InetAddressExtManual};
use socket2::*;

fn create_socket(domain: Domain, protocol: Protocol) -> Result<gio::Socket, Box<dyn Error>> {
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

/// Send a single ping to `ip_address`.
async fn ping(ip_address: IpAddr) -> Result<bool, Box<dyn Error>> {
    log::trace!("Sending ICMP echo request to {ip_address}");
    let (domain, protocol) = match ip_address {
        IpAddr::V4(_) => (Domain::IPV4, Protocol::ICMPV4),
        IpAddr::V6(_) => (Domain::IPV6, Protocol::ICMPV6),
    };
    let socket = create_socket(domain, protocol)?;
    let condition = socket
        .create_source_future(IOCondition::OUT, Cancellable::NONE, glib::Priority::DEFAULT)
        .await;
    if condition != glib::IOCondition::OUT {
        socket.close().ok();
        return Ok(false);
    }

    let condition =
        socket.create_source_future(IOCondition::IN, Cancellable::NONE, glib::Priority::DEFAULT);
    let socket_address: gio::InetSocketAddress = SocketAddr::new(ip_address, 0).into();
    let header = IcmpEchoHeader { id: 42, seq: 23 };
    let payload = b"wakeup-ping wakeup-ping wakeup-ping wakeup-ping";
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
        return Ok(false);
    }

    let mut buffer = [0; 128];
    let (bytes_received, _) = socket.receive_from(&mut buffer, Cancellable::NONE)?;
    socket.close().ok();
    match ip_address {
        IpAddr::V4(_) => {
            let response = Icmpv4Slice::from_slice(&buffer[..bytes_received])?;
            Ok(matches!(response.icmp_type(), Icmpv4Type::EchoReply(_)))
        }
        IpAddr::V6(_) => {
            let response = Icmpv6Slice::from_slice(&buffer[..bytes_received])?;
            Ok(matches!(response.icmp_type(), Icmpv6Type::EchoReply(_)))
        }
    }
}

/// Resolve a `host` to one or more IP addresses.
pub async fn resolve_host(host: &str) -> Result<Vec<IpAddr>, Box<dyn Error>> {
    let addresses = gio::Resolver::default().lookup_by_name_future(host).await?;
    if addresses.is_empty() {
        Err(std::io::Error::new(std::io::ErrorKind::NotFound, "No addresses found").into())
    } else {
        Ok(addresses.into_iter().map(to_rust).collect())
    }
}

/// Monitor a target at the given `interval`.
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
                // Resolve the target to an IP address
                let addresses = match state.take() {
                    Some(address) => {
                        log::trace!("Using cached IP address {address}");
                        Ok(vec![address])
                    }
                    None => match target {
                        Target::Dns(ref host) => {
                            log::trace!("Resolving {host} to IP address");
                            resolve_host(host).await
                        }
                        Target::Addr(ip_addr) => Ok(vec![ip_addr]),
                    },
                };
                match addresses {
                    Ok(addresses) => {
                        let mut replying_addresses = addresses
                            .into_iter()
                            .map(|addr| ping(addr).map(move |result| (addr, result)))
                            .collect::<FuturesUnordered<_>>()
                            .filter_map(|(ip_address, result)| match result {
                                Ok(true) => {
                                    log::trace!("{ip_address} replied");
                                    future::ready(Some(ip_address))
                                }
                                Ok(false) => {
                                    log::trace!("{ip_address} did not reply");
                                    future::ready(None)
                                }
                                Err(error) => {
                                    log::trace!("Failed to ping {ip_address}: {error}");
                                    future::ready(None)
                                }
                            });
                        let online_address = select_biased! {
                            address = replying_addresses.select_next_some() => {
                                Some(address)
                            }
                            // TODO: Move timeout upwards, to include name resolution!
                            _ = glib::timeout_future(interval).fuse() => {
                                None
                            }
                        };
                        match online_address {
                            Some(address) => {
                                state.replace(Some(address));
                                Some(true)
                            }
                            None => Some(false),
                        }
                    }
                    Err(error) => {
                        log::trace!("Failed to resolve {target} to an IP address: {error}");
                        Some(false)
                    }
                }
            }
        })
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use gtk::gio;

    use crate::net::to_rust;

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
}

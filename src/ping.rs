// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! A dead simple and somewhat stupid ping implementation.

use std::cell::RefCell;
use std::error::Error;
use std::fmt::Display;
use std::net::{IpAddr, SocketAddr};
use std::os::fd::{AsRawFd, OwnedFd};
use std::rc::Rc;
use std::time::Duration;

use etherparse::{IcmpEchoHeader, Icmpv4Slice, Icmpv4Type, Icmpv6Slice, Icmpv6Type};
use futures_util::{select_biased, FutureExt, Stream, StreamExt};
use glib::IOCondition;
use gtk::gio::{self, Cancellable};
use gtk::prelude::{CancellableExt, ResolverExt, SocketExt, SocketExtManual};
use socket2::*;

use crate::log::G_LOG_DOMAIN;

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

/// Send a single ping to `ip_address`.
async fn ping(ip_address: IpAddr, cancellable: &Cancellable) -> Result<bool, Box<dyn Error>> {
    glib::trace!("Sending ICMP echo request to {ip_address}");
    let (domain, protocol) = match ip_address {
        IpAddr::V4(_) => (Domain::IPV4, Protocol::ICMPV4),
        IpAddr::V6(_) => (Domain::IPV6, Protocol::ICMPV6),
    };
    let socket = create_socket(domain, protocol)?;
    let condition = socket
        .create_source_future(IOCondition::OUT, Some(cancellable), glib::Priority::DEFAULT)
        .await;
    if condition != glib::IOCondition::OUT {
        socket.close().ok();
        return Ok(false);
    }

    let condition =
        socket.create_source_future(IOCondition::IN, Some(cancellable), glib::Priority::DEFAULT);
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
    let bytes_written = socket.send_to(Some(&socket_address), &packet, Some(cancellable))?;
    assert!(bytes_written == packet.len());
    if condition.await != glib::IOCondition::IN {
        socket.close().ok();
        return Ok(false);
    }

    let mut buffer = [0; 128];
    let (bytes_received, _) = socket.receive_from(&mut buffer, Some(cancellable))?;
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

/// Resolve a `host` to an IP address.
pub async fn resolve_host(host: &str) -> Result<IpAddr, Box<dyn Error>> {
    // TODO: lookup_by_name_future does not take a cancellable?
    let addresses = gio::Resolver::default().lookup_by_name_future(host).await?;
    let first_address = addresses
        .first()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "No addresses found"))?;
    Ok(first_address.clone().into())
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
                let address = match state.take() {
                    Some(address) => {
                        glib::trace!("Using cached IP address {address}");
                        Ok(address)
                    }
                    None => match target {
                        Target::Dns(ref host) => {
                            glib::trace!("Resolving {host} to IP address");
                            resolve_host(host).await.inspect(|ip_addr| {
                                glib::trace!("Resolved {host} to {ip_addr}");
                            })
                        }
                        Target::Addr(ip_addr) => Ok(ip_addr),
                    },
                };
                match address {
                    Ok(ip_address) => {
                        let cancellable = Cancellable::new();
                        let is_online = select_biased! {
                            response = ping(ip_address, &cancellable).fuse() => {
                                match response {
                                    Ok(true) => {
                                        glib::trace!("{ip_address} replied");
                                        true
                                    },
                                    Ok(false) => {
                                        glib::trace!("{ip_address} did not reply");
                                        false
                                    },
                                    Err(error) => {
                                        glib::trace!("Failed to ping {ip_address}: {error}");
                                        false
                                    },
                                }
                            }
                            _ = glib::timeout_future(interval).fuse() => {
                                glib::trace!("{ip_address} did not respond within {interval:#?}");
                                cancellable.cancel();
                                false
                            }
                        };
                        if is_online {
                            // The target system replied so let's remember its IP address for the next ping
                            state.replace(Some(ip_address));
                        }
                        Some(is_online)
                    }
                    Err(error) => {
                        glib::trace!("Failed to resolve {target} to an IP address: {error}");
                        Some(false)
                    }
                }
            }
        })
}

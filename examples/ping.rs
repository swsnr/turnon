// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::error::Error;
use std::net::{IpAddr, SocketAddr};
use std::os::unix::io::IntoRawFd;
use std::{io::Write, time::Duration};

use etherparse::{IcmpEchoHeader, Icmpv4Slice, Icmpv4Type, Icmpv6Slice, Icmpv6Type};
use futures_util::{select_biased, FutureExt, StreamExt};
use gtk::gio::{self, Cancellable};
use gtk::glib::IOCondition;
use gtk::prelude::{CancellableExt, ResolverExt, SocketExt, SocketExtManual};
use socket2::*;

fn create_socket(domain: Domain, protocol: Protocol) -> Result<gio::Socket, Box<dyn Error>> {
    let socket = socket2::Socket::new_raw(domain, Type::DGRAM, Some(protocol))?;
    socket.set_nonblocking(true)?;
    socket.set_read_timeout(Some(Duration::from_secs(10)))?;
    Ok(unsafe { gio::Socket::from_fd(socket.into_raw_fd()) }?)
}

#[derive(Debug, Clone)]
enum Source {
    Dns(String),
    Addr(IpAddr),
}

async fn ping(source: Source, cancellable: &Cancellable) -> Result<bool, Box<dyn Error>> {
    let ip_address = match source {
        Source::Dns(name) => {
            let addresses = gio::Resolver::default()
                .lookup_by_name_future(&name)
                .await?;
            addresses.first().map(|a| a.clone().into()).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "No addresses found")
            })?
        }
        Source::Addr(ip_addr) => ip_addr,
    };
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

fn main() {
    let host = std::env::args().nth(1).unwrap();
    let source = host
        .parse::<IpAddr>()
        .map_or_else(|_| Source::Dns(host), Source::Addr);
    dbg!(&source);

    let main_loop = glib::MainLoop::new(None, false);

    let ping = futures_util::stream::iter(vec![()])
        .chain(glib::interval_stream_seconds(5))
        .inspect(|_| {
            let mut out = std::io::stdout().lock();
            out.write_all(b"o").unwrap();
            out.flush().unwrap();
        })
        .for_each(move |_| {
            let source = source.clone();
            async move {
                let mut out = std::io::stdout().lock();
                let cancellable = Cancellable::new();
                select_biased! {
                    response = ping(source, &cancellable).fuse() => {
                        match response {
                            Ok(is_online) => {
                                let c = if is_online { "X" } else { "_" };
                                out.write_all(c.as_bytes()).unwrap();
                            }
                            Err(_) => {
                                out.write_all(b"!").unwrap();
                            }
                        }
                    }
                    _ = glib::timeout_future_seconds(5).fuse() => {
                        cancellable.cancel();
                        out.write_all(b"X").unwrap();

                    }
                }
                out.flush().unwrap();
            }
        });

    glib::spawn_future_local(ping);

    main_loop.run();
}

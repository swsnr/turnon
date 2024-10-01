// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::cell::Cell;
use std::net::{IpAddr, SocketAddr};
use std::os::unix::io::IntoRawFd;
use std::rc::Rc;
use std::{io::Write, time::Duration};

use etherparse::{IcmpEchoHeader, Icmpv4Slice, Icmpv4Type};
use glib::object::Cast;
use gtk::gio::{self, Cancellable};
use gtk::prelude::SocketExtManual;
use socket2::*;

fn create_icmp_ping(echo_header: IcmpEchoHeader) -> Vec<u8> {
    let echo = etherparse::Icmpv4Type::EchoRequest(echo_header);
    let payload = b"wakeup-ping wakeup-ping wakeup-ping wakeup-ping";
    let header = etherparse::Icmpv4Header::with_checksum(echo, payload);
    let mut packet = header.to_bytes().to_vec();
    packet.extend_from_slice(payload);
    packet
}

fn create_socket() -> gio::Socket {
    let socket =
        socket2::Socket::new_raw(Domain::IPV4, Type::DGRAM, Some(Protocol::ICMPV4)).unwrap();
    socket.set_nonblocking(true).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    unsafe { gio::Socket::from_fd(socket.into_raw_fd()) }.unwrap()
}

fn send_icmp_echo(address: &gio::InetSocketAddress, socket: &gio::Socket) {
    let mut out = std::io::stdout().lock();
    out.write_all(b"o").unwrap();
    out.flush().unwrap();
    let packet = create_icmp_ping(IcmpEchoHeader { id: 42, seq: 23 });
    match socket.send_to(Some(address), &packet, Cancellable::NONE) {
        Ok(bytes_written) => {
            assert!(bytes_written == packet.len())
        }
        Err(err_) => {
            out.write_all(b"!").unwrap();
            out.flush().unwrap();
        }
    }
}

fn main() {
    let main_loop = glib::MainLoop::new(None, false);
    let is_online = Rc::new(Cell::new(false));
    glib::timeout_add_local(
        Duration::from_millis(500),
        glib::clone!(
            #[strong]
            is_online,
            move || {
                let c = if is_online.get() { "X" } else { "_" };
                let mut out = std::io::stdout().lock();
                out.write_all(c.as_bytes()).unwrap();
                out.flush().unwrap();
                glib::ControlFlow::Continue
            }
        ),
    );

    let host = std::env::args().nth(1).unwrap();

    let ping_interval = Duration::from_secs(5);
    let socket = create_socket();
    let ip_address = host.parse::<IpAddr>().unwrap();
    let address = SocketAddr::new(ip_address, 0);
    let gio_socket_address: gio::InetSocketAddress = address.into();
    glib::timeout_add_local(
        Duration::ZERO,
        glib::clone!(
            #[strong]
            socket,
            #[strong]
            gio_socket_address,
            move || {
                send_icmp_echo(&gio_socket_address, &socket);
                glib::ControlFlow::Break
            }
        ),
    );
    glib::timeout_add_local(
        ping_interval,
        glib::clone!(
            #[strong]
            socket,
            #[strong]
            gio_socket_address,
            move || {
                send_icmp_echo(&gio_socket_address, &socket);
                glib::ControlFlow::Continue
            }
        ),
    );

    let mut running_timer: Option<glib::SourceId> = None;
    socket
        .create_source(
            glib::IOCondition::IN,
            Cancellable::NONE,
            None,
            glib::Priority::DEFAULT,
            glib::clone!(
                #[strong]
                socket,
                move |_, condition| {
                    if condition == glib::IOCondition::IN {
                        let mut buffer = [0; 128];
                        let (bytes_received, remote_address) =
                            socket.receive_from(&mut buffer, Cancellable::NONE).unwrap();
                        let remote_address: SocketAddr = remote_address
                            .downcast::<gio::InetSocketAddress>()
                            .unwrap()
                            .into();
                        let response = Icmpv4Slice::from_slice(&buffer[..bytes_received]).unwrap();
                        // We're online if we get an echo reply from the destination host
                        is_online.set(
                            matches!(response.icmp_type(), Icmpv4Type::EchoReply(_))
                                && remote_address == address,
                        );
                        if let Some(id) = running_timer.take() {
                            id.remove();
                        }
                        if is_online.get() {
                            running_timer = Some(glib::timeout_add_local(
                                ping_interval * 2,
                                glib::clone!(
                                    #[strong]
                                    is_online,
                                    move || {
                                        is_online.set(false);
                                        glib::ControlFlow::Break
                                    }
                                ),
                            ));
                        }
                    }
                    glib::ControlFlow::Continue
                }
            ),
        )
        .attach(None);

    main_loop.run();
}

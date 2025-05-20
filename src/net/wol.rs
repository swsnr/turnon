// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

//! Wake On LAN (magic packet) implementation.

use std::net::{Ipv4Addr, SocketAddr};

use glib::IOCondition;
use gtk::gio::Cancellable;
use gtk::gio::prelude::{SocketExt, SocketExtManual};
use gtk::gio::{self, IOErrorEnum};
use macaddr::MacAddr6;

/// Send a magic Wake On LAN packet to the given `mac_address`.
///
/// Sends the magic package as UDP package to port 9 on the IPv4 broadcast address.
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
        return Err(glib::Error::new(
            IOErrorEnum::BrokenPipe,
            &format!("Socket for waking {mac_address} not ready to write"),
        ));
    }
    let mut payload = [0; 102];
    wol::fill_magic_packet(&mut payload, mac_address);
    let broadcast_and_discard_address: gio::InetSocketAddress =
        SocketAddr::new(Ipv4Addr::BROADCAST.into(), 9).into();
    let bytes_sent = socket.send_to(
        Some(&broadcast_and_discard_address),
        payload,
        Cancellable::NONE,
    )?;
    assert!(bytes_sent == payload.len());
    Ok(())
}

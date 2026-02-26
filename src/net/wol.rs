// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

//! Wake On LAN (magic packet) implementation.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use glib::IOCondition;
use gtk::gio::Cancellable;
use gtk::gio::prelude::{SocketExt, SocketExtManual};
use gtk::gio::{self, IOErrorEnum};
use wol::MacAddress;

/// The default target address for magic packets.
///
/// This provides the broadcast IPv4 address on port 9 which is a reasonable default for magic packets.
pub const WOL_DEFAULT_TARGET_ADDRESS: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::BROADCAST, 9);

/// Send a magic Wake On LAN packet to the given `mac_address`.
///
/// Sends the magic package as UDP package to `target_address`.
pub async fn wol(mac_address: MacAddress, target_address: SocketAddr) -> Result<(), glib::Error> {
    let socket = gio::Socket::new(
        match target_address {
            SocketAddr::V4(_) => gio::SocketFamily::Ipv4,
            SocketAddr::V6(_) => gio::SocketFamily::Ipv6,
        },
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
    let bytes_sent = socket.send_to(
        Some(&gio::InetSocketAddress::from(target_address)),
        payload,
        Cancellable::NONE,
    )?;
    assert!(bytes_sent == payload.len());
    Ok(())
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use std::{
        net::{IpAddr, Ipv4Addr, Ipv6Addr, UdpSocket},
        time::Duration,
    };

    use glib::async_test;

    async fn assert_wol_packet(address: IpAddr) {
        let server = UdpSocket::bind((address, 0)).unwrap();
        server
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let target_address = server.local_addr().unwrap();

        assert_eq!(target_address.ip(), address);
        assert!(0 < target_address.port());

        // 0x0E is a local MAC address, so it's unlikely to match any actual MAC address of any device on the current system.
        let mac_address = [0x0E, 0x12, 0x13, 0x14, 0x15, 0x16].into();
        let result = super::wol(mac_address, target_address).await;
        assert!(result.is_ok(), "Result: {result:?}");

        let mut expected_package = [0; 102];
        wol::fill_magic_packet(&mut expected_package, mac_address);

        let mut buffer = [0; 1024];
        let size = server.recv(&mut buffer).unwrap();
        assert_eq!(&buffer[..size], expected_package.as_slice());
    }

    #[async_test]
    async fn send_real_wol_packet() {
        assert_wol_packet(Ipv4Addr::LOCALHOST.into()).await;
        assert_wol_packet(Ipv6Addr::LOCALHOST.into()).await;
    }
}

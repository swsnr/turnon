// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Access the Linux ARP cache.

use std::fmt::Display;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::ErrorKind;
use std::net::{AddrParseError, Ipv4Addr};
use std::num::ParseIntError;
use std::path::Path;
use std::str::FromStr;

use bitflags::bitflags;
use macaddr::MacAddr6;

/// A ARP hardware type.
///
/// See <https://github.com/torvalds/linux/blob/v6.12/include/uapi/linux/if_arp.h#L29>
/// for known hardware types as of Linux 6.12.
///
/// We do not represent all hardware types, but only those we're interested in
/// with regards to TurnOn.
#[derive(Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum ArpKnownHardwareType {
    // Ethernet (including WiFi)
    Ether = 1,
}

/// A known or unknown hardware type.
#[derive(Debug, PartialEq, Eq)]
pub enum ArpHardwareType {
    /// A hardware type we know.
    Known(ArpKnownHardwareType),
    /// A hardware type we do not understand.
    Unknown(u16),
}

impl FromStr for ArpHardwareType {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use ArpHardwareType::*;
        let hw_type = match u16::from_str_radix(s, 16)? {
            1 => Known(ArpKnownHardwareType::Ether),
            other => Unknown(other),
        };
        Ok(hw_type)
    }
}

bitflags! {
    /// Flags for ARP cache entries.
    ///
    /// See <https://github.com/torvalds/linux/blob/v6.12/include/uapi/linux/if_arp.h#L132>
    /// for known flags as of Linux 6.12.
    #[derive(Debug, Eq, PartialEq)]
    pub struct ArpCacheEntryFlags: u8 {
        /// completed entry (ha valid)
        const ATF_COM = 0x02;
        /// permanent entry
        const ATF_PERM = 0x04;
        /// publish entry
        const ATF_PUBL = 0x08;
        /// has requested trailers
        const ATF_USETRAILERS = 0x10;
        /// want to use a netmask (only for proxy entries)
        const ATF_NETMASK = 0x20;
        /// don't answer this addresses
        const ATF_DONTPUB = 0x40;
    }
}

impl FromStr for ArpCacheEntryFlags {
    type Err = ParseIntError;

    /// Parse flags, discarding unknown flags.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ArpCacheEntryFlags::from_bits_truncate(u8::from_str(s)?))
    }
}

/// An ARP cache entry.
#[derive(Debug)]
pub struct ArpCacheEntry {
    /// The IP address.
    pub ip_address: Ipv4Addr,
    /// The hardware type.
    pub hardware_type: ArpHardwareType,
    /// Internal flags for this cache entry.
    pub flags: ArpCacheEntryFlags,
    /// The hardware address for this entry.
    pub hardware_address: MacAddr6,
}

#[derive(Debug)]
pub enum ArpCacheParseError {
    MissingCell(&'static str, u8),
    InvalidIpAddress(AddrParseError),
    InvalidHardwareType(ParseIntError),
    InvalidFlags(ParseIntError),
    InvalidHardwareAddess(macaddr::ParseError),
}

impl Display for ArpCacheParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArpCacheParseError::MissingCell(cell, index) => {
                write!(f, "Missing cell {cell} at index {index}")
            }
            ArpCacheParseError::InvalidIpAddress(addr_parse_error) => {
                write!(f, "Failed to parse IP address: {addr_parse_error}")
            }
            ArpCacheParseError::InvalidHardwareType(parse_int_error) => {
                write!(f, "Invalid hardware type: {parse_int_error}")
            }
            ArpCacheParseError::InvalidFlags(parse_int_error) => {
                write!(f, "Invalid flags: {parse_int_error}")
            }
            ArpCacheParseError::InvalidHardwareAddess(parse_error) => {
                write!(f, "Failed to parse hardware address: {parse_error}")
            }
        }
    }
}

impl std::error::Error for ArpCacheParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ArpCacheParseError::InvalidIpAddress(addr_parse_error) => Some(addr_parse_error),
            ArpCacheParseError::InvalidHardwareType(parse_int_error) => Some(parse_int_error),
            ArpCacheParseError::InvalidFlags(parse_int_error) => Some(parse_int_error),
            _ => None,
        }
    }
}

impl From<AddrParseError> for ArpCacheParseError {
    fn from(value: AddrParseError) -> Self {
        ArpCacheParseError::InvalidIpAddress(value)
    }
}

impl From<macaddr::ParseError> for ArpCacheParseError {
    fn from(value: macaddr::ParseError) -> Self {
        ArpCacheParseError::InvalidHardwareAddess(value)
    }
}

impl FromStr for ArpCacheEntry {
    type Err = ArpCacheParseError;

    /// Parse an ARP cache entry from one line of `/proc/net/arp`.
    ///
    /// See `proc_net(5)` for some details.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use ArpCacheParseError::*;
        let mut parts = s.trim_ascii().split_ascii_whitespace();
        let ip_address = Ipv4Addr::from_str(parts.next().ok_or(MissingCell("IP address", 0))?)?;
        let hardware_type = ArpHardwareType::from_str(
            parts
                .next()
                .ok_or(MissingCell("HW type", 1))?
                .trim_start_matches("0x"),
        )
        .map_err(InvalidHardwareType)?;
        let flags = ArpCacheEntryFlags::from_str(
            parts
                .next()
                .ok_or(MissingCell("Flags", 2))?
                .trim_start_matches("0x"),
        )
        .map_err(InvalidFlags)?;
        let hardware_address =
            MacAddr6::from_str(parts.next().ok_or(MissingCell("HW address", 3))?)?;
        // The cache table also has mask and device columns, but we don't care for these
        Ok(ArpCacheEntry {
            ip_address,
            hardware_type,
            flags,
            hardware_address,
        })
    }
}

pub fn read_arp_cache<R: BufRead>(
    reader: R,
) -> impl Iterator<Item = std::io::Result<ArpCacheEntry>> {
    reader
        .lines()
        .skip(1) // skip over the headling line
        .map(|l| {
            l.and_then(|l| {
                ArpCacheEntry::from_str(&l)
                    .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e))
            })
        })
}

pub fn read_arp_cache_from_path<P: AsRef<Path>>(
    path: P,
) -> std::io::Result<impl Iterator<Item = std::io::Result<ArpCacheEntry>>> {
    let source = BufReader::new(std::fs::File::open(path)?);
    Ok(read_arp_cache(source))
}

pub fn read_linux_arp_cache(
) -> std::io::Result<impl Iterator<Item = std::io::Result<ArpCacheEntry>>> {
    read_arp_cache_from_path("/proc/net/arp")
}

#[cfg(test)]
mod tests {
    use std::{net::Ipv4Addr, str::FromStr};

    use macaddr::MacAddr6;

    use super::*;

    #[test]
    pub fn test_arp_cache_entry_from_str() {
        let entry = ArpCacheEntry::from_str(
            "192.168.178.130  0x1         0x2         b6:a3:b0:48:80:f1     *        wlp4s0
",
        )
        .unwrap();
        assert_eq!(entry.ip_address, Ipv4Addr::new(192, 168, 178, 130));
        assert_eq!(
            entry.hardware_type,
            ArpHardwareType::Known(ArpKnownHardwareType::Ether)
        );
        assert_eq!(entry.flags, ArpCacheEntryFlags::ATF_COM);
        assert_eq!(
            entry.hardware_address,
            MacAddr6::new(0xb6, 0xa3, 0xb0, 0x48, 0x80, 0xf1)
        );
    }
}

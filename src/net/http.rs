// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

use std::fmt::Display;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;

use gtk::gio::prelude::SocketClientExt;
use gtk::gio::{IOErrorEnum, SocketClient};

use crate::config::G_LOG_DOMAIN;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpScheme {
    Http,
    Https,
}

impl HttpScheme {
    pub fn as_str(self) -> &'static str {
        match self {
            HttpScheme::Http => "http:",
            HttpScheme::Https => "https:",
        }
    }

    pub fn port(self) -> u16 {
        match self {
            HttpScheme::Http => 80,
            HttpScheme::Https => 443,
        }
    }

    pub fn to_url(self, host: &str) -> String {
        if IpAddr::from_str(host).is_ok_and(|ip| ip.is_ipv6()) {
            format!("{self}//[{host}]")
        } else {
            format!("{self}//{host}")
        }
    }
}

impl Display for HttpScheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Probe a host for HTTP support.
///
/// Attempt to connect to `host` via standard HTTPS and HTTP ports 443 and 80
/// respectively, and return the supported scheme if either succeeds.
pub async fn probe_http(host: &str, timeout: Duration) -> Option<HttpScheme> {
    for scheme in [HttpScheme::Https, HttpScheme::Http] {
        let client = SocketClient::new();
        client.set_tls(matches!(scheme, HttpScheme::Https));
        // We probe hosts we can ping directly, so there should never be a proxy involved.
        client.set_enable_proxy(false);
        client.set_timeout(u32::try_from(timeout.as_secs()).unwrap());
        let result =
            glib::future_with_timeout(timeout, client.connect_to_host_future(host, scheme.port()))
                .await
                .map_err(|_| {
                    glib::Error::new(IOErrorEnum::TimedOut, &format!("Host {host} timed out"))
                })
                .and_then(|r| r)
                .inspect_err(|error| {
                    glib::info!("{host} not reachable via {scheme}: {error}");
                });
        if result.is_ok() {
            glib::info!("{host} reachable via {scheme}");
            return Some(scheme);
        }
    }
    None
}

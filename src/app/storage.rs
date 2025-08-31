// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

use std::fs::File;
use std::io::{ErrorKind, Result};
use std::net::SocketAddrV4;
use std::panic::resume_unwind;
use std::path::{Path, PathBuf};

use async_channel::{Receiver, Sender};
use glib::object::Cast;
use gtk::gio::ListStore;
use macaddr::MacAddr6;
use serde::{Deserialize, Serialize};

use crate::config::G_LOG_DOMAIN;
use crate::net::{MacAddr6Boxed, SocketAddrV4Boxed, WOL_DEFAULT_TARGET_ADDRESS};

use super::model::Device;

/// A stored device.
///
/// Like [`model::Device`], but for serialization.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredDevice {
    pub label: String,
    #[serde(with = "mac_addr6_as_string")]
    pub mac_address: MacAddr6,
    pub host: String,
    /// The target address.
    ///
    /// Optional for compatibility with serialized data from previous releases.
    pub target_address: Option<SocketAddrV4>,
}

impl From<&StoredDevice> for Device {
    fn from(device: &StoredDevice) -> Self {
        Device::new(
            &device.label,
            MacAddr6Boxed::from(device.mac_address),
            &device.host,
            SocketAddrV4Boxed::from(device.target_address.unwrap_or(WOL_DEFAULT_TARGET_ADDRESS)),
        )
    }
}

impl From<StoredDevice> for Device {
    fn from(device: StoredDevice) -> Self {
        Device::from(&device)
    }
}

impl From<&Device> for StoredDevice {
    fn from(device: &Device) -> Self {
        StoredDevice {
            label: device.label(),
            host: device.host(),
            mac_address: *device.mac_address(),
            target_address: Some(*device.target_address()),
        }
    }
}

impl From<Device> for StoredDevice {
    fn from(device: Device) -> Self {
        StoredDevice::from(&device)
    }
}

mod mac_addr6_as_string {
    use std::str::FromStr;

    use macaddr::MacAddr6;
    use serde::{Deserialize, Deserializer, Serializer};

    #[allow(
        clippy::trivially_copy_pass_by_ref,
        reason = "serde's with requires a &T type here"
    )]
    pub fn serialize<S>(addr: &MacAddr6, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&addr.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<MacAddr6, D::Error>
    where
        D: Deserializer<'de>,
    {
        MacAddr6::from_str(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

fn read_devices(target: &Path) -> Result<Vec<StoredDevice>> {
    File::open(target).and_then(|source| {
        serde_json::from_reader(source)
            .map_err(|err| std::io::Error::new(ErrorKind::InvalidData, err))
    })
}

fn write_devices(target: &Path, devices: &[StoredDevice]) -> Result<()> {
    let target_directory = target.parent().ok_or(std::io::Error::new(
        ErrorKind::InvalidData,
        format!("Path {} must be absolute, but wasn't!", target.display()),
    ))?;
    std::fs::create_dir_all(target_directory)?;
    File::create(target).and_then(|sink| {
        serde_json::to_writer_pretty(sink, &devices)
            .map_err(|err| std::io::Error::new(ErrorKind::InvalidData, err))
    })
}

async fn handle_save_requests(data_file: PathBuf, rx: Receiver<Vec<StoredDevice>>) {
    loop {
        if let Ok(devices) = rx.recv().await {
            let target = data_file.clone();
            // Off-load serialization and writing to gio's blocking pool. We
            // then wait for the result of saving the file before processing
            // the next storage request, to avoid writing to the same file
            // in parallel.
            let result =
                gtk::gio::spawn_blocking(move || write_devices(&target, devices.as_slice())).await;
            match result {
                Err(payload) => {
                    resume_unwind(payload);
                }
                Ok(Err(error)) => {
                    glib::error!(
                        "Failed to save devices to {}: {}",
                        data_file.display(),
                        error
                    );
                }
                Ok(Ok(())) => {
                    glib::info!("Saved devices to {}", data_file.display());
                }
            }
        } else {
            glib::warn!("Channel closed");
            break;
        }
    }
}

/// A service which can save devices.
///
/// This service processes requests to save a serialized list of devices to a
/// file.
///
/// It allows many concurrent save requests, but always discards all but the
/// latest save requests, to avoid redundant writes to the file.
#[derive(Debug)]
pub struct StorageService {
    target: PathBuf,
    tx: Sender<Vec<StoredDevice>>,
    rx: Receiver<Vec<StoredDevice>>,
}

impl StorageService {
    /// Create a new storage service for the given `target` file.
    pub fn new(target: PathBuf) -> Self {
        // Create a bounded channel which can only hold a single request at a time.
        // Then we can use force_send to overwrite earlier storage requests to avoid
        // redundant writes.
        let (tx, rx) = async_channel::bounded::<Vec<StoredDevice>>(1);
        Self { target, tx, rx }
    }

    /// Get the target path for storage.
    pub fn target(&self) -> &Path {
        &self.target
    }

    /// Get a client for this service.
    pub fn client(&self) -> StorageServiceClient {
        StorageServiceClient {
            tx: self.tx.clone(),
        }
    }

    /// Load devices synchronously from storage.
    pub fn load_sync(&self) -> Result<Vec<StoredDevice>> {
        read_devices(&self.target)
    }

    /// Spawn the service.
    ///
    /// Consumes the service to ensure that only a single service instance is
    /// running.
    ///
    /// After spawning no further clients can be created from the service.  You
    /// must create a client first, and then clone that client.
    pub async fn spawn(self) {
        handle_save_requests(self.target, self.rx).await;
    }
}

/// A storage client which can request saving devices from a storage service.
///
/// The client is cheap to clone.
#[derive(Debug, Clone)]
pub struct StorageServiceClient {
    tx: Sender<Vec<StoredDevice>>,
}

impl StorageServiceClient {
    /// Request that the service save all devices in the given device `model`.
    pub fn request_save_device_store(&self, model: &ListStore) {
        self.request_save_devices(
            model
                .into_iter()
                .filter_map(|obj| obj.unwrap().downcast::<Device>().ok())
                .map(StoredDevice::from)
                .collect(),
        );
    }

    /// Request that the service save the given `devices`.
    pub fn request_save_devices(&self, devices: Vec<StoredDevice>) {
        // Forcibly overwrite earlier storage requests, to ensure we only store
        // the most recent version of data.
        self.tx.force_send(devices).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;

    /// Baseline deserialization: The earliest data format of all Turn On versions.
    #[test]
    fn deserialize_device_baseline() {
        let json = r#"
{
    "label": "NAS",
    "mac_address": "2E:E3:50:A3:E2:F7",
    "host": "192.168.2.100"
}"#;
        let device = serde_json::from_str::<StoredDevice>(json).unwrap();
        assert_eq!(device.label, "NAS");
        assert_eq!(device.host, "192.168.2.100");
        assert_eq!(device.mac_address.to_string(), "2E:E3:50:A3:E2:F7");
        assert!(device.target_address.is_none());
    }

    #[test]
    fn deserialize_with_target_address() {
        let json = r#"
{
    "label": "spam",
    "mac_address": "2E:E3:60:A3:E2:F7",
    "host": "foo",
    "target_address": "192.168.2.3:9"
}"#;
        let device = serde_json::from_str::<StoredDevice>(json).unwrap();
        assert_eq!(device.label, "spam");
        assert_eq!(device.host, "foo");
        assert_eq!(device.mac_address.to_string(), "2E:E3:60:A3:E2:F7");
        assert_eq!(
            device.target_address,
            Some(SocketAddrV4::new(Ipv4Addr::new(192, 168, 2, 3), 9))
        );
    }

    #[test]
    fn device_from_stored_device() {
        let stored_device = StoredDevice {
            label: "foo-server".into(),
            mac_address: MacAddr6::new(0x10, 0x11, 0x12, 0x13, 0x14, 0x15),
            host: "123.456.789.100".into(),
            target_address: None,
        };
        let device = Device::from(&stored_device);
        assert_eq!(device.label(), stored_device.label);
        assert_eq!(*device.mac_address(), stored_device.mac_address);
        assert_eq!(device.host(), stored_device.host);
        assert_eq!(*device.target_address(), WOL_DEFAULT_TARGET_ADDRESS);

        let target_address = SocketAddrV4::new(Ipv4Addr::new(123, 231, 123, 231), 42);
        let stored_device = StoredDevice {
            label: "foo".into(),
            mac_address: MacAddr6::new(0x20, 0x21, 0x22, 0x23, 0x24, 0x25),
            host: "foo.example.com".into(),
            target_address: Some(target_address),
        };
        let device = Device::from(&stored_device);
        assert_eq!(device.label(), stored_device.label);
        assert_eq!(*device.mac_address(), stored_device.mac_address);
        assert_eq!(device.host(), stored_device.host);
        assert_eq!(*device.target_address(), target_address);
    }

    #[test]
    fn stored_device_from_device() {
        let device = Device::new(
            "foo",
            MacAddr6::new(0x0a, 0x0b, 0x0c, 0x0a, 0x0b, 0x0c).into(),
            "spam.example.com",
            SocketAddrV4::new(Ipv4Addr::new(123, 231, 123, 231), 42).into(),
        );
        let stored_device = StoredDevice::from(&device);
        assert_eq!(
            stored_device,
            StoredDevice {
                label: device.label(),
                mac_address: *device.mac_address(),
                host: device.host(),
                target_address: Some(*device.target_address())
            }
        );
    }
}

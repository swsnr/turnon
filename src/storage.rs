// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fs::File;
use std::io::{ErrorKind, Result};
use std::panic::resume_unwind;
use std::path::{Path, PathBuf};

use async_channel::{Receiver, Sender};
use macaddr::MacAddr6;
use serde::{Deserialize, Serialize};

use crate::config::G_LOG_DOMAIN;

/// A stored device.
///
/// Like [`model::Device`], but for serialization.
#[derive(Debug, Serialize, Deserialize)]
pub struct StoredDevice {
    pub label: String,
    #[serde(with = "mac_addr6_as_string")]
    pub mac_address: MacAddr6,
    pub host: String,
}

mod mac_addr6_as_string {
    use std::str::FromStr;

    use macaddr::MacAddr6;
    use serde::{Deserialize, Deserializer, Serializer};

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

fn write_devices(target: &Path, devices: Vec<StoredDevice>) -> Result<()> {
    std::fs::create_dir_all(target.parent().expect("Target path not absolute?")).ok();
    File::create(target).and_then(|sink| {
        serde_json::to_writer_pretty(sink, &devices)
            .map_err(|err| std::io::Error::new(ErrorKind::InvalidData, err))
    })
}

async fn handle_save_requests(data_file: PathBuf, rx: Receiver<Vec<StoredDevice>>) {
    let pool = glib::ThreadPool::shared(Some(1)).unwrap();
    loop {
        match rx.recv().await {
            Ok(devices) => {
                let target = data_file.clone();
                // Off-load serialization and writing to the thread pool. We
                // then wait for the result of saving the file before processing
                // the next storage request, to avoid writing to the same file
                // in parallel.
                let result = pool
                    .push_future(move || write_devices(&target, devices))
                    .unwrap()
                    .await;
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
                    Ok(Ok(_)) => {
                        glib::info!("Saved devices to {}", data_file.display());
                    }
                }
            }
            Err(_) => {
                glib::warn!("Channel closed");
                break;
            }
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
        handle_save_requests(self.target, self.rx).await
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
    /// Request that the service save the given `devices`.
    pub fn request_save_devices(&self, devices: Vec<StoredDevice>) {
        // Forcibly overwrite earlier storage requests, to ensure we only store
        // the most recent version of data.
        self.tx.force_send(devices).unwrap();
    }
}

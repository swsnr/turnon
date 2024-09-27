// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fs::File;
use std::io::{ErrorKind, Result};
use std::panic::resume_unwind;
use std::path::PathBuf;

use gtk::glib;
use macaddr::MacAddr6;
use serde::{Deserialize, Serialize};

/// A device
#[derive(Debug, Serialize, Deserialize)]
pub struct StoredDevice {
    pub id: String,
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

pub struct DevicesStorage {
    io_pool: glib::ThreadPool,
    location: PathBuf,
}

fn read_devices(target: PathBuf) -> Result<Vec<StoredDevice>> {
    File::open(target).and_then(|source| {
        serde_json::from_reader(source)
            .map_err(|err| std::io::Error::new(ErrorKind::InvalidData, err))
    })
}

fn write_devices(target: PathBuf, devices: Vec<StoredDevice>) -> Result<()> {
    std::fs::create_dir_all(target.parent().expect("Target path not absolute?")).ok();
    File::create(target).and_then(|sink| {
        serde_json::to_writer_pretty(sink, &devices)
            .map_err(|err| std::io::Error::new(ErrorKind::InvalidData, err))
    })
}

impl DevicesStorage {
    pub fn new(location: PathBuf) -> Self {
        Self {
            io_pool: glib::ThreadPool::shared(None).unwrap(),
            location,
        }
    }

    /// Load devices from this storage.
    pub async fn load(&self) -> Result<Vec<StoredDevice>> {
        let target = self.location.clone();
        let result = self
            .io_pool
            .push_future(move || read_devices(target))
            .expect("Failed to load on thread pool")
            .await;
        match result {
            Ok(devices) => devices,
            Err(panicked) => resume_unwind(panicked),
        }
    }

    pub async fn save(&self, devices: Vec<StoredDevice>) -> Result<()> {
        let target = self.location.clone();
        let result = self
            .io_pool
            .push_future(move || write_devices(target, devices))
            .expect("Failed to save on thread pool")
            .await;
        if let Err(panicked) = result {
            resume_unwind(panicked)
        }
        Ok(())
    }
}

// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Dummy module for i18n.
//!
//! Until we have gettext we use no-op functions in this module to mark translatable strings.

pub fn gettext<T: Into<String>>(msgid: T) -> String {
    msgid.into()
}

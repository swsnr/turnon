// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::path::PathBuf;

/// Compile all blueprint files.
fn compile_blueprint() {
    let blueprint_files: Vec<PathBuf> = glob::glob("resources/**/*.blp")
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    for blueprint_file in &blueprint_files {
        println!("cargo:rerun-if-changed={}", blueprint_file.display());
    }

    let output = std::process::Command::new("blueprint-compiler")
        .args(["batch-compile", "resources", "resources"])
        .args(&blueprint_files)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "blueprint-compiler failed with exit status {} and stdout\n{}\n\n and stderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn main() {
    if let Some("1") | Some("true") = std::env::var("SKIP_BLUEPRINT").ok().as_deref() {
        println!("cargo::warning=Skipping blueprint compilation, falling back to committed files.");
    } else {
        // Since blueprint generates UI files for glib resources, we must compile
        // blueprint files before compiling resources!
        compile_blueprint();
    }

    glib_build_tools::compile_resources(
        &["resources"],
        "resources/resources.gresource.xml",
        "wakeup.gresource",
    );
}

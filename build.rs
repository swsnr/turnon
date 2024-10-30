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

    if let Some("1") | Some("true") = std::env::var("SKIP_BLUEPRINT").ok().as_deref() {
        println!("cargo::warning=Skipping blueprint compilation, falling back to committed files.");
        return;
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

fn msgfmt_desktop() {
    let desktop_file = "de.swsnr.turnon.desktop.in";
    println!("cargo:rerun-if-changed={}", desktop_file);

    let output = std::process::Command::new("msgfmt")
        .args([
            "--desktop",
            "--template",
            desktop_file,
            "-d",
            "po",
            "--output",
            "de.swsnr.turnon.desktop",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "msgfmt failed with exit status {} and stdout\n{}\n\n and stderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn msgfmt_metainfo() {
    let metainfo_file = "resources/de.swsnr.turnon.metainfo.xml.in";
    println!("cargo:rerun-if-changed={}", metainfo_file);

    let output = std::process::Command::new("msgfmt")
        .args([
            "--xml",
            "--template",
            metainfo_file,
            "-d",
            "po",
            "--output",
            "resources/de.swsnr.turnon.metainfo.xml",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "msgfmt failed with exit status {} and stdout\n{}\n\n and stderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn msgfmt() {
    let po_files: Vec<PathBuf> = glob::glob("po/*.po")
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    for po_file in &po_files {
        println!("cargo:rerun-if-changed={}", po_file.display());
    }
    println!("cargo:rerun-if-changed=po/LINGUAS");

    let msgfmt_exists = std::process::Command::new("msgfmt")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success());

    if msgfmt_exists {
        msgfmt_desktop();
        msgfmt_metainfo();
    } else {
        println!("cargo::warning=msgfmt not found; using untranslated desktop and metainfo file.");
        for file in [
            "resources/de.swsnr.turnon.metainfo.xml",
            "de.swsnr.turnon.desktop",
        ] {
            std::fs::copy(format!("{file}.in"), file).unwrap();
        }
    }
}

fn main() {
    // Compile blueprints and msgfmt our metainfo template first, as these are
    // inputs to resource compilation.
    compile_blueprint();
    msgfmt();

    glib_build_tools::compile_resources(
        &["resources"],
        "resources/resources.gresource.xml",
        "turnon.gresource",
    );
}

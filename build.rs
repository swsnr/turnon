// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

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

    let output = Command::new("blueprint-compiler")
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

/// Run `msgfmt` over a template file to merge translations with the template.
fn msgfmt_template<P: AsRef<Path>>(template: P) {
    let target = template.as_ref().with_extension("");
    println!("cargo:rerun-if-changed={}", template.as_ref().display());

    let mode = match target.extension().and_then(|e| e.to_str()) {
        Some("desktop") => "--desktop",
        Some("xml") => "--xml",
        other => panic!("Unsupported template extension: {:?}", other),
    };

    let output = Command::new("msgfmt")
        .args([mode, "--template"])
        .arg(template.as_ref())
        .args(["-d", "po", "--output"])
        .arg(target)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "msgfmt of {} failed with exit status {} and stdout\n{}\n\n and stderr:\n{}",
        template.as_ref().display(),
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

    let msgfmt_exists = Command::new("msgfmt")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success());

    let templates = [
        "resources/de.swsnr.turnon.metainfo.xml.in",
        "de.swsnr.turnon.desktop.in",
    ];
    if msgfmt_exists {
        for file in templates {
            msgfmt_template(file);
        }
    } else {
        println!("cargo::warning=msgfmt not found; using untranslated desktop and metainfo file.");
        for file in templates {
            std::fs::copy(file, Path::new(file).with_extension("")).unwrap();
        }
    }
}

pub fn compile_resources<P: AsRef<Path>>(source_dirs: &[P], gresource: &str, target: &str) {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);

    let mut command = Command::new("glib-compile-resources");

    for source_dir in source_dirs {
        command.arg("--sourcedir").arg(source_dir.as_ref());
    }

    let output = command
        .arg("--target")
        .arg(out_dir.join(target))
        .arg(gresource)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "glib-compile-resources failed with exit status {} and stderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    println!("cargo:rerun-if-changed={gresource}");
    let mut command = Command::new("glib-compile-resources");

    for source_dir in source_dirs {
        command.arg("--sourcedir").arg(source_dir.as_ref());
    }

    let output = command
        .arg("--generate-dependencies")
        .arg(gresource)
        .output()
        .unwrap()
        .stdout;
    for line in String::from_utf8(output).unwrap().lines() {
        let dep = Path::new(line);
        if let Some("ui") = dep.extension().and_then(|e| e.to_str()) {
            // We build UI files from blueprint, so adapt the dependency
            println!(
                "cargo:rerun-if-changed={}",
                dep.with_extension("blp").display()
            );
        } else if line.ends_with(".metainfo.xml") {
            // We build the metainfo file from the template
            println!("cargo:rerun-if-changed={line}.in",);
        } else {
            println!("cargo:rerun-if-changed={line}",);
        }
    }
}

fn main() {
    // Compile blueprints and msgfmt our metainfo template first, as these are
    // inputs to resource compilation.
    compile_blueprint();
    msgfmt();
    compile_resources(
        &["resources"],
        "resources/resources.gresource.xml",
        "turnon.gresource",
    );
}

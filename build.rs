// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn glob_io(pattern: &str) -> Result<Vec<PathBuf>> {
    glob::glob(pattern)
        .map_err(|err| Error::new(ErrorKind::Other, err))?
        .map(|item| item.map_err(|err| Error::new(ErrorKind::Other, err)))
        .collect::<Result<Vec<PathBuf>>>()
}

trait CommandExt {
    fn checked(&mut self);
}

impl CommandExt for Command {
    fn checked(&mut self) {
        let status = self.status().unwrap();
        if !status.success() {
            panic!("Command {:?} failed with status {status}", self);
        }
    }
}

/// Compile all blueprint files.
fn compile_blueprint() -> Vec<PathBuf> {
    let blueprint_files = glob_io("resources/**/*.blp").unwrap();
    if let Some("1") | Some("true") = std::env::var("SKIP_BLUEPRINT").ok().as_deref() {
        println!("cargo::warning=Skipping blueprint compilation, falling back to committed files.");
    } else {
        Command::new("blueprint-compiler")
            .args(["batch-compile", "resources", "resources"])
            .args(&blueprint_files)
            .checked();
    }
    blueprint_files
}

/// Run `msgfmt` over a template file to merge translations with the template.
fn msgfmt_template<P: AsRef<Path>>(template: P) {
    let target = template.as_ref().with_extension("");
    let mode = match target.extension().and_then(|e| e.to_str()) {
        Some("desktop") => "--desktop",
        Some("xml") => "--xml",
        other => panic!("Unsupported template extension: {:?}", other),
    };

    Command::new("msgfmt")
        .args([mode, "--template"])
        .arg(template.as_ref())
        .args(["-d", "po", "--output"])
        .arg(target)
        .checked();
}

fn msgfmt() -> Vec<PathBuf> {
    let po_files: Vec<PathBuf> = glob::glob("po/*.po")
        .unwrap()
        .collect::<std::result::Result<_, _>>()
        .unwrap();

    let msgfmt_exists = Command::new("msgfmt")
        .arg("--version")
        .status()
        .is_ok_and(|status| status.success());

    let templates = &[
        Path::new("resources/de.swsnr.turnon.metainfo.xml.in").to_owned(),
        Path::new("de.swsnr.turnon.desktop.in").to_owned(),
    ];
    if msgfmt_exists {
        for file in templates {
            msgfmt_template(file);
        }
    } else {
        println!("cargo::warning=msgfmt not found; using untranslated desktop and metainfo file.");
        for file in templates {
            std::fs::copy(file, file.with_extension("")).unwrap();
        }
    }

    let mut sources = po_files;
    sources.push("po/LINGUAS".into());
    sources.extend_from_slice(templates);
    sources
}

pub fn compile_resources<P: AsRef<Path>>(
    source_dirs: &[P],
    gresource: &str,
    target: &str,
) -> Vec<PathBuf> {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);

    let mut command = Command::new("glib-compile-resources");

    for source_dir in source_dirs {
        command.arg("--sourcedir").arg(source_dir.as_ref());
    }

    command
        .arg("--target")
        .arg(out_dir.join(target))
        .arg(gresource)
        .checked();

    let mut command = Command::new("glib-compile-resources");
    for source_dir in source_dirs {
        command.arg("--sourcedir").arg(source_dir.as_ref());
    }

    let output = command
        .arg("--generate-dependencies")
        .arg(gresource)
        .stderr(Stdio::inherit())
        .output()
        .unwrap()
        .stdout;

    let mut sources = vec![Path::new(gresource).into()];

    for line in String::from_utf8(output).unwrap().lines() {
        if line.ends_with(".ui") {
            // We build UI files from blueprint, so adapt the dependency
            sources.push(Path::new(line).with_extension("blp"))
        } else if line.ends_with(".metainfo.xml") {
            sources.push(Path::new(line).with_extension("xml.in"));
        } else {
            sources.push(line.into());
        }
    }

    sources
}

fn glib_compile_schemas() -> Vec<PathBuf> {
    let schemas = glob_io("schemas/*.gschema.xml").unwrap();
    Command::new("glib-compile-schemas")
        .args(["--strict", "schemas"])
        .checked();
    schemas
}

fn main() {
    let tasks = [
        std::thread::spawn(compile_blueprint),
        std::thread::spawn(glib_compile_schemas),
        std::thread::spawn(msgfmt),
    ];

    let mut sources = tasks
        .into_iter()
        .flat_map(|task| task.join().unwrap())
        .collect::<Vec<_>>();

    sources.extend_from_slice(
        compile_resources(
            &["resources"],
            "resources/resources.gresource.xml",
            "turnon.gresource",
        )
        .as_slice(),
    );

    for source in sources {
        println!("cargo:rerun-if-changed={}", source.display());
    }
}

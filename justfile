
APPID := 'de.swsnr.turnon'

xgettext_opts := '--package-name=' + APPID + \
    ' --foreign-user --copyright-holder "Sebastian Wiesner <sebastian@swsnr.de>"' + \
    ' --sort-by-file --from-code=UTF-8 --add-comments'

default:
    just --list

vet *ARGS:
    @# Only consider Linux dependencies, as that's all I care for.
    @# Seems to be unofficial, see https://github.com/mozilla/cargo-vet/issues/579, but works
    env CARGO_BUILD_TARGET=x86_64-unknown-linux-gnu cargo vet {{ARGS}}

# Remove build files from source code tree
clean:
    rm -fr po/*.mo builddir repo .flatpak-builder

lint-blueprint:
    blueprint-compiler format resources/**/*.blp

lint-rust:
    cargo +stable deny --all-features --locked check
    cargo +stable fmt -- --check
    cargo +stable clippy --all-targets

lint-flatpak:
    flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest flatpak/de.swsnr.turnon.yaml
    flatpak run --command=flatpak-builder-lint org.flatpak.Builder appstream resources/de.swsnr.turnon.metainfo.xml

lint-data:
    appstreamcli validate --explain resources/de.swsnr.turnon.metainfo.xml

lint-all: lint-rust lint-blueprint lint-data lint-flatpak

test-rust:
    cargo +stable build
    cargo +stable test

test-all: (vet "--locked") lint-all test-rust

# Extract the message template from all source files.
pot:
    find src -name '*.rs' > po/POTFILES.rs
    find resources/ -name '*.blp' > po/POTFILES.blp
    xgettext {{xgettext_opts}} --language=C --keyword=dpgettext2:2c,3 --files-from=po/POTFILES.rs --output=po/de.swsnr.turnon.rs.pot
    xgettext {{xgettext_opts}} --language=C --keyword=_ --keyword=C_:1c,2 --files-from=po/POTFILES.blp --output=po/de.swsnr.turnon.blp.pot
    xgettext {{xgettext_opts}} --output=po/de.swsnr.turnon.pot \
        po/de.swsnr.turnon.rs.pot po/de.swsnr.turnon.blp.pot \
        resources/de.swsnr.turnon.metainfo.xml.in de.swsnr.turnon.desktop.in
    rm -f po/POTFILES* po/de.swsnr.turnon.rs.pot po/de.swsnr.turnon.blp.pot
    @# We strip the POT-Creation-Date from the resulting POT because xgettext bumps
    @# it everytime regardless if anything else changed, and this just generates
    @# needless diffs.
    sed -i /POT-Creation-Date/d po/de.swsnr.turnon.pot

# Build and install development flatpak without sandboxing
flatpak-devel-install:
    flatpak run org.flatpak.Builder --force-clean --user --install \
        --install-deps-from=flathub --repo=repo \
        builddir flatpak/de.swsnr.turnon.Devel.yaml

# Lint the flatpak repo (you must run flatpak-build first)
lint-flatpak-repo:
    flatpak run --command=flatpak-builder-lint org.flatpak.Builder repo repo

# Build (but not install) regular flatpak
flatpak-build: && lint-flatpak-repo
    flatpak run org.flatpak.Builder --force-clean --sandbox \
        --install-deps-from=flathub --ccache \
        --mirror-screenshots-url=https://dl.flathub.org/media/ --repo=repo \
        builddir flatpak/de.swsnr.turnon.yaml

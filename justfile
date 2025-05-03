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

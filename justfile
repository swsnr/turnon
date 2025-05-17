
APPID := 'de.swsnr.turnon'
# The destination prefix to install files to.  Combines traditional DESTDIR and
# PREFIX variables; turnon does not encode the prefix into its binary and thus
# does not need to distinguish between the prefix and the destdir.
DESTPREFIX := '/app'

xgettext_opts := '--package-name=' + APPID + \
    ' --foreign-user --copyright-holder "Sebastian Wiesner <sebastian@swsnr.de>"' + \
    ' --sort-by-file --from-code=UTF-8 --add-comments'

version := `git describe`

default:
    just --list

vet *ARGS:
    cargo vet {{ARGS}}

# Remove build files from source code tree
clean:
    rm -fr build .flatpak-builder .flatpak-builddir .flatpak-repo

# Compile all blueprint files to UI files.
compile-blueprint:
    mkdir -p build/resources-src/
    blueprint-compiler batch-compile build/resources-src/ resources resources/**/*.blp

# Compile the translated metainfo file.
compile-metainfo:
    mkdir -p build/resources-src/
    msgfmt --xml --template de.swsnr.turnon.metainfo.xml -d po --output build/de.swsnr.turnon.metainfo.xml
    @# Also add the translated metainfo file to resources
    cp -t build/resources-src build/de.swsnr.turnon.metainfo.xml

# Compile all Glib resources for this application.
compile-resources: compile-blueprint compile-metainfo
    mkdir -p build/resources
    glib-compile-resources --sourcedir=build/resources-src \
        --target build/resources/resources.generated.gresource \
        resources/resources.generated.gresource.xml
    glib-compile-resources --sourcedir=resources \
        --target build/resources/resources.data.gresource \
        resources/resources.data.gresource.xml

# Compile the translated desktop file.
compile-desktop-file:
    mkdir -p build
    msgfmt --desktop --template de.swsnr.turnon.desktop -d po --output build/de.swsnr.turnon.desktop

# Compile the settings schema
compile-schemas:
    mkdir -p build/schemas
    cp -t build/schemas de.swsnr.turnon.gschema.xml
    glib-compile-schemas --strict build/schemas

# Compile all extra files (resources, settings schemas, etc.)
compile: compile-resources compile-desktop-file compile-schemas

lint-blueprint:
    blueprint-compiler format resources/**/*.blp

lint-rust:
    cargo +stable deny --all-features --locked check
    cargo +stable fmt -- --check
    cargo +stable clippy --all-targets

lint-flatpak:
    flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest flatpak/de.swsnr.turnon.yaml
    flatpak run --command=flatpak-builder-lint org.flatpak.Builder appstream de.swsnr.turnon.metainfo.xml

lint-data:
    appstreamcli validate --strict --explain de.swsnr.turnon.metainfo.xml

lint-all: lint-rust lint-blueprint lint-data lint-flatpak

test-rust:
    cargo +stable build
    cargo +stable test

test-all: (vet "--locked") lint-all compile test-rust

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
        --install-deps-from=flathub --repo=.flatpak-repo \
        .flatpak-builddir flatpak/de.swsnr.turnon.Devel.yaml

# Build (but not install) regular flatpak
flatpak-build:
    flatpak run org.flatpak.Builder --force-clean --sandbox \
        --install-deps-from=flathub --ccache --user \
        --mirror-screenshots-url=https://dl.flathub.org/media/ --repo=.flatpak-repo \
        .flatpak-builddir flatpak/de.swsnr.turnon.yaml

# Print release notes
print-release-notes:
    @appstreamcli metainfo-to-news --format yaml resources/de.swsnr.turnon.metainfo.xml.in - | \
        yq eval-all '[.]' -oj | jq -r --arg tag "{{version}}" \
        '.[] | select(.Version == ($tag | ltrimstr("v"))) | .Description | tostring'

# Update the flatpak manifest for `version`.
flatpak-update-manifest:
    flatpak run --command=flatpak-cargo-generator org.flatpak.Builder \
        <(git --no-pager show '{{version}}:Cargo.lock') -o flatpak/de.swsnr.turnon.cargo-sources.json
    yq eval -i '.modules.[1].sources.[0].tag = "$TAG_NAME"' flatpak/de.swsnr.turnon.yaml
    yq eval -i '.modules.[1].sources.[0].commit = "$TAG_COMMIT"' flatpak/de.swsnr.turnon.yaml
    env TAG_NAME='{{version}}' \
        TAG_COMMIT="$(git rev-parse '{{version}}')" \
        yq eval -i '(.. | select(tag == "!!str")) |= envsubst' flatpak/de.swsnr.turnon.yaml
    git add flatpak/de.swsnr.turnon.yaml flatpak/de.swsnr.turnon.cargo-sources.json
    @git commit -m 'Update flatpak manifest for {{version}}'
    @echo "Run git push and trigger sync workflow at https://github.com/flathub/de.swsnr.turnon/actions/workflows/sync.yaml"

_post-release:
    @echo "Create new release at https://codeberg.org/swsnr/turnon/tags"
    @echo "Run 'just print-release-notes' to get Markdown release notes for the release"
    @echo "Run 'just flatpak-update-manifest' to update the flatpak manifest."

release *ARGS: test-all && _post-release
    cargo release {{ARGS}}

# Patch files for the Devel build
patch-devel:
    sed -Ei 's/^version = "([^"]+)"/version = "\1+{{version}}"/' Cargo.toml
    cargo update -p turnon
    sed -i '/{{APPID}}/! s/de\.swsnr\.turnon/{{APPID}}/g' \
        src/config.rs \
        de.swsnr.turnon.metainfo.xml de.swsnr.turnon.desktop \
        dbus-1/de.swsnr.turnon.service de.swsnr.turnon.search-provider.ini \
        de.swsnr.turnon.gschema.xml

_install-po po_file:
    install -dm0755 '{{DESTPREFIX}}/share/locale/{{file_stem(po_file)}}/LC_MESSAGES'
    msgfmt -o '{{DESTPREFIX}}/share/locale/{{file_stem(po_file)}}/LC_MESSAGES/{{APPID}}.mo' '{{po_file}}'

# Install to DESTPREFIX (run just compile and cargo build --release first!).
install:
    @# Install all message catalogs
    find po/ -name '*.po' -exec just version= DESTPREFIX='{{DESTPREFIX}}' APPID='{{APPID}}' _install-po '{}' ';'
    @# Install cargo build --release binary
    install -Dm0755 target/release/turnon '{{DESTPREFIX}}/bin/{{APPID}}'
    @# Install translated appstream metadata and desktop file
    install -Dm0644 build/de.swsnr.turnon.metainfo.xml '{{DESTPREFIX}}/share/metainfo/{{APPID}}.metainfo.xml'
    install -Dm0644 build/de.swsnr.turnon.desktop '{{DESTPREFIX}}/share/applications/{{APPID}}.desktop'
    @# Install static files (icons, etc.)
    install -Dm0644 -t '{{DESTPREFIX}}/share/icons/hicolor/scalable/apps/' 'resources/icons/scalable/apps/{{APPID}}.svg'
    install -Dm0644 resources/icons/symbolic/apps/de.swsnr.turnon-symbolic.svg \
        ''{{DESTPREFIX}}/share/icons/hicolor/symbolic/apps/{{APPID}}-symbolic.svg''
    install -Dm0644 dbus-1/de.swsnr.turnon.service '{{DESTPREFIX}}/share/dbus-1/services/{{APPID}}.service'
    install -Dm0644 de.swsnr.turnon.search-provider.ini '{{DESTPREFIX}}/share/gnome-shell/search-providers/{{APPID}}.search-provider.ini'
    install -Dm0644 de.swsnr.turnon.gschema.xml '{{DESTPREFIX}}/share/glib-2.0/schemas/{{APPID}}.gschema.xml'
    @# Compile settings schemas after installation
    glib-compile-schemas --strict '{{DESTPREFIX}}/share/glib-2.0/schemas'

# Assemble the README image from screenshots.
build-social-image:
    montage -geometry 602x602+19+19 \
        screenshots/start-page.png screenshots/list-of-discovered-devices.png \
        social-image.png
    oxipng social-image.png

# Run with default settings to make screenshots
run-for-screenshot devices_file='screenshots/devices.json':
    @# Run app with default settings: Force the in-memory gsettings backend to
    @# block access to standard Gtk settings, and tell Adwaita not to access
    @# portals to prevent it from getting dark mode and accent color from desktop
    @# settings.
    @#
    @# Effectively this makes our app run with default settings.
    env GSETTINGS_BACKEND=memory ADW_DISABLE_PORTAL=1 \
        cargo run -- --devices-file '{{devices_file}}' --arp-cache-file screenshots/arp

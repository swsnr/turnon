APPID := 'de.swsnr.turnon.Devel'

# The destination prefix to install files to.  Combines traditional DESTDIR and
# PREFIX variables; turnon does not encode the prefix into its binary and thus
# does not need to distinguish between the prefix and the destdir.
DESTPREFIX := '/app'

xgettext_opts := '--package-name=' + trim_end_matches(APPID, '.Devel') + \
    ' --foreign-user --copyright-holder "Sebastian Wiesner <sebastian@swsnr.de>"' + \
    ' --sort-by-file --from-code=UTF-8 --add-comments'

version := `git describe`

default:
    just --list

# Remove build files from source code tree
clean:
    rm -fr build .flatpak-builder .flatpak-builddir .flatpak-repo

# Write APP ID to file for build
configure-app-id:
    @rm -f build/app-id
    @mkdir -p build
    @# Do not add a newline; that'd break the app ID in include_str!
    echo -n '{{APPID}}' > build/app-id

# Compile all blueprint files to UI files.
compile-blueprint:
    mkdir -p build/resources-src/
    blueprint-compiler batch-compile build/resources-src/ resources resources/**/*.blp resources/*.blp

# Compile the translated metainfo file.
compile-metainfo:
    mkdir -p build/resources-src/
    msgfmt --xml --template de.swsnr.turnon.metainfo.xml -d po --output build/{{APPID}}.metainfo.xml
    @# Patch the app ID
    sed -i '/{{APPID}}/! s/de\.swsnr\.turnon/{{APPID}}/g' build/{{APPID}}.metainfo.xml
    @# Also add the translated metainfo file to resources
    cp build/{{APPID}}.metainfo.xml build/resources-src/de.swsnr.turnon.metainfo.xml

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
    msgfmt --desktop --template de.swsnr.turnon.desktop -d po --output build/{{APPID}}.desktop
    @# Patch the app ID
    sed -i '/{{APPID}}/! s/de\.swsnr\.turnon/{{APPID}}/g' build/{{APPID}}.desktop

# Compile the settings schema
compile-schemas:
    @mkdir -p build/schemas
    cp de.swsnr.turnon.gschema.xml build/schemas/{{APPID}}.gschema.xml
    @# Patch the app ID
    sed -i '/{{APPID}}/! s/de\.swsnr\.turnon/{{APPID}}/g' build/schemas/{{APPID}}.gschema.xml
    glib-compile-schemas --strict build/schemas

# Compile misc files to patch the app ID
compile-misc:
    @mkdir -p build
    cp dbus-1/de.swsnr.turnon.service build/{{APPID}}.service
    cp de.swsnr.turnon.search-provider.ini build/{{APPID}}.search-provider.ini
    sed -i '/{{APPID}}/! s/de\.swsnr\.turnon/{{APPID}}/g' \
        build/{{APPID}}.service build/{{APPID}}.search-provider.ini

# Compile all extra files (resources, settings schemas, etc.)
compile: configure-app-id compile-resources compile-desktop-file compile-schemas compile-misc

lint-blueprint:
    blueprint-compiler format resources/**/*.blp resources/*.blp

lint-rust:
    cargo +stable vet --locked
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

test-all: lint-all compile test-rust

# Extract the message template from all source files.
pot:
    find src -name '*.rs' > po/POTFILES.rs
    find resources/ -name '*.blp' > po/POTFILES.blp
    xgettext {{xgettext_opts}} --language=Rust --keyword=dpgettext2:2c,3 --files-from=po/POTFILES.rs --output=po/de.swsnr.turnon.rs.pot
    xgettext {{xgettext_opts}} --language=C --keyword=_ --keyword=C_:1c,2 --files-from=po/POTFILES.blp --output=po/de.swsnr.turnon.blp.pot
    xgettext {{xgettext_opts}} --output=po/de.swsnr.turnon.pot \
        po/de.swsnr.turnon.rs.pot po/de.swsnr.turnon.blp.pot \
        de.swsnr.turnon.metainfo.xml de.swsnr.turnon.desktop
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
    @appstreamcli metainfo-to-news --format yaml de.swsnr.turnon.metainfo.xml - | \
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

# Patch `version` into `Cargo.toml`
patch-version:
    sed -Ei 's/^version = "([^"]+)"/version = "\1+{{version}}"/' Cargo.toml
    cargo update -p turnon

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
    install -Dm0644 -t '{{DESTPREFIX}}/share/metainfo/' build/{{APPID}}.metainfo.xml
    install -Dm0644 -t '{{DESTPREFIX}}/share/applications/' build/{{APPID}}.desktop
    @# Install static files (icons, etc.)
    install -Dm0644 -t '{{DESTPREFIX}}/share/dbus-1/services/' build/{{APPID}}.service
    install -Dm0644 -t '{{DESTPREFIX}}/share/gnome-shell/search-providers/' build/{{APPID}}.search-provider.ini
    install -Dm0644 -t '{{DESTPREFIX}}/share/glib-2.0/schemas/' build/schemas/{{APPID}}.gschema.xml
    install -Dm0644 -t '{{DESTPREFIX}}/share/icons/hicolor/scalable/apps/' 'resources/icons/scalable/apps/{{APPID}}.svg'
    install -Dm0644 resources/icons/symbolic/apps/de.swsnr.turnon-symbolic.svg \
        '{{DESTPREFIX}}/share/icons/hicolor/symbolic/apps/{{APPID}}-symbolic.svg'
    @# Compile settings schemas after installation
    glib-compile-schemas --strict '{{DESTPREFIX}}/share/glib-2.0/schemas'

# Assemble the README image from screenshots.
build-social-image:
    montage -geometry 602x602+19+19 \
        screenshots/start-page.png screenshots/list-of-discovered-devices.png \
        social-image.png
    oxipng social-image.png

# Run oxipng on all screenshots
optimize-screenshots:
    oxipng screenshots/*.png

# Run the app repeatedly to make screenshots
run-for-screenshots: && optimize-screenshots build-social-image
    @# Run app with default settings: Force the in-memory gsettings backend to
    @# block access to standard Gtk settings, and tell Adwaita not to access
    @# portals to prevent it from getting dark mode and accent color from desktop
    @# settings.
    @#
    @# Effectively this makes our app run with default settings.
    @#
    @echo -e "\n$(tput bold)Standard window size, fake devices: Screenshot of devices list and discovered devices$(tput sgr0)"
    @env GSETTINGS_BACKEND=memory ADW_DISABLE_PORTAL=1 \
        cargo run --quiet -- --devices-file 'screenshots/devices.json' --arp-cache-file screenshots/arp
    @echo -e "\n$(tput bold)Larger window size, fake devices: Screenshot of edit dialog for first device$(tput sgr0)"
    @env GSETTINGS_BACKEND=memory ADW_DISABLE_PORTAL=1 \
        cargo run --quiet -- --devices-file 'screenshots/devices.json' --arp-cache-file screenshots/arp --main-window-height 640
    @echo -e "\n$(tput bold)Standard window size, no devices: Screenshot of startpage$(tput sgr0)"
    @env GSETTINGS_BACKEND=memory ADW_DISABLE_PORTAL=1 \
        cargo run --quiet -- --devices-file /dev/null --arp-cache-file screenshots/arp

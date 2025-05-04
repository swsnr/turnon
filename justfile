
APPID := 'de.swsnr.turnon'
# The destination prefix to install files to.  Combines traditional DESTDIR and
# PREFIX variables; turnon does not encode the prefix into its binary and thus
# does not need to distinguish between the prefix and the destdir.
DESTPREFIX := '/app'

xgettext_opts := '--package-name=' + APPID + \
    ' --foreign-user --copyright-holder "Sebastian Wiesner <sebastian@swsnr.de>"' + \
    ' --sort-by-file --from-code=UTF-8 --add-comments'

version := `git describe`
release_archive := 'turnon-' + version + '.tar.zst'
release_vendor_archive := 'turnon-' + version + '-vendor.tar.zst'

default:
    just --list

vet *ARGS:
    @# Only consider Linux dependencies, as that's all I care for.
    @# Seems to be unofficial, see https://github.com/mozilla/cargo-vet/issues/579, but works
    env CARGO_BUILD_TARGET=x86_64-unknown-linux-gnu cargo vet {{ARGS}}

# Remove build files from source code tree
clean:
    rm -fr builddir repo .flatpak-builder

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

_dist:
    rm -rf dist
    mkdir dist

# Build and sign a reproducible archive of cargo vendor sources
_vendor: _dist
    rm -rf vendor/
    cargo vendor --locked
    echo SOURCE_DATE_EPOCH="$(env LC_ALL=C TZ=UTC0 git show --quiet --date='format-local:%Y-%m-%dT%H:%M:%SZ' --format="%cd" HEAD)"
    # See https://reproducible-builds.org/docs/archives/
    env LC_ALL=C TZ=UTC0 tar --numeric-owner --owner 0 --group 0 \
        --sort name --mode='go+u,go-w' --format=posix \
        --pax-option=exthdr.name=%d/PaxHeaders/%f \
        --pax-option=delete=atime,delete=ctime \
        --mtime="$(env LC_ALL=C TZ=UTC0 git show --quiet --date='format-local:%Y-%m-%dT%H:%M:%SZ' --format="%cd" HEAD)" \
        -c -f "dist/{{release_vendor_archive}}" \
        --zstd vendor

# Build and sign a reproducible git archive bundle
_git-archive: _dist
    env LC_ALL=C TZ=UTC0 git archive --format tar \
        --prefix "{{without_extension(without_extension(release_archive))}}/" \
        --output "dist/{{without_extension(release_archive)}}" HEAD
    zstd --rm "dist/{{without_extension(release_archive)}}"

_release_notes: _dist
    appstreamcli metainfo-to-news resources/de.swsnr.turnon.metainfo.xml.in dist/news.yaml
    yq eval-all '[.]' -oj dist/news.yaml > dist/news.json
    jq -r --arg tag "$(git describe)" '.[] | select(.Version == ($tag | ltrimstr("v"))) | .Description | tostring' > dist/relnotes.md < dist/news.json
    rm dist/news.{json,yaml}

package: _git-archive _vendor _release_notes
    curl https://codeberg.org/swsnr.keys > dist/key
    ssh-keygen -Y sign -f dist/key -n file "dist/{{release_archive}}"
    ssh-keygen -Y sign -f dist/key -n file "dist/{{release_vendor_archive}}"
    rm dist/key

flatpak-update-manifest:
    yq eval -i '.modules.[0].sources.[0].url = "https://codeberg.org/swsnr/turnon/releases/download/$TAG_NAME/turnon-$TAG_NAME.tar.zst"' flatpak/de.swsnr.turnon.yaml
    yq eval -i '.modules.[0].sources.[0].sha256 = "$ARCHIVE_SHA256"' flatpak/de.swsnr.turnon.yaml
    yq eval -i '.modules.[0].sources.[1].url = "https://codeberg.org/swsnr/turnon/releases/download/$TAG_NAME/turnon-$TAG_NAME-vendor.tar.zst"' flatpak/de.swsnr.turnon.yaml
    yq eval -i '.modules.[0].sources.[1].sha256 = "$VENDOR_SHA256"' flatpak/de.swsnr.turnon.yaml
    env TAG_NAME="{{version}}" \
        ARCHIVE_SHA256={{sha256_file('dist' / release_archive)}} \
        VENDOR_SHA256={{sha256_file('dist' / release_vendor_archive)}} \
        yq eval -i '(.. | select(tag == "!!str")) |= envsubst' flatpak/de.swsnr.turnon.yaml
    git add flatpak/de.swsnr.turnon.yaml
    git commit -m 'Update flatpak manifest for {{version}}'
    @echo "Run git push and trigger sync workflow at https://github.com/flathub/de.swsnr.turnon/actions/workflows/sync.yaml"

_post-release:
    @echo "Run just package to create dist archives."
    @echo "Create new release at https://codeberg.org/swsnr/turnon/tags"
    @echo "Use dist/relnotes.md as release body"
    @echo "Attach archives and signatures in dist as release body"
    @echo "Then run just flatpak-update-manifest to update the flatpak manifest."

release *ARGS: test-all && _post-release
    cargo release {{ARGS}}

# Patch files for the Devel build
patch-devel:
    sed -Ei 's/^version = "([^"]+)"/version = "\1+{{version}}"/' Cargo.toml
    cargo update -p turnon
    sed -i '/{{APPID}}/! s/de\.swsnr\.turnon/{{APPID}}/g' \
        src/config.rs \
        resources/de.swsnr.turnon.metainfo.xml.in de.swsnr.turnon.desktop.in \
        dbus-1/de.swsnr.turnon.service de.swsnr.turnon.search-provider.ini \
        schemas/de.swsnr.turnon.gschema.xml

_install-po po_file:
    install -dm0755 '{{DESTPREFIX}}/share/locale/{{file_stem(po_file)}}/LC_MESSAGES'
    msgfmt -o '{{DESTPREFIX}}/share/locale/{{file_stem(po_file)}}/LC_MESSAGES/{{APPID}}.mo' '{{po_file}}'

install:
    find po/ -name '*.po' -exec just DESTPREFIX='{{DESTPREFIX}}' APPID='{{APPID}}' _install-po '{}' ';'
    install -Dm0755 target/release/turnon '{{DESTPREFIX}}/bin/{{APPID}}'
    install -Dm0644 -t '{{DESTPREFIX}}/share/icons/hicolor/scalable/apps/' 'resources/icons/scalable/apps/{{APPID}}.svg'
    install -Dm0644 resources/icons/symbolic/apps/de.swsnr.turnon-symbolic.svg \
        ''{{DESTPREFIX}}/share/icons/hicolor/symbolic/apps/{{APPID}}-symbolic.svg''
    install -Dm0644 de.swsnr.turnon.desktop '{{DESTPREFIX}}/share/applications/{{APPID}}.desktop'
    install -Dm0644 resources/de.swsnr.turnon.metainfo.xml '{{DESTPREFIX}}/share/metainfo/{{APPID}}.metainfo.xml'
    install -Dm0644 dbus-1/de.swsnr.turnon.service '{{DESTPREFIX}}/share/dbus-1/services/{{APPID}}.service'
    install -Dm0644 de.swsnr.turnon.search-provider.ini '{{DESTPREFIX}}/share/gnome-shell/search-providers/{{APPID}}.search-provider.ini'
    install -Dm0644 schemas/de.swsnr.turnon.gschema.xml '{{DESTPREFIX}}/share/glib-2.0/schemas/{{APPID}}.gschema.xml'
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

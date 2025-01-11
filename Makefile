# The app ID to use.
#
# Use de.swsnr.turnon for the standard app ID, and de.swsnr.turnon.Devel to
# build a nightly snapshot.  Other values are not supported.
APPID = de.swsnr.turnon
# The destination prefix to install files to.  Combines traditional DESTDIR and
# PREFIX variables; turnon does not encode the prefix into its binary and thus
# does not need to distinguish between the prefix and the destdir.
DESTPREFIX = /app
# Installation directory for locale files.
LOCALEDIR = $(DESTPREFIX)/share/locale/

GIT_DESCRIBE = $(shell git describe)

BLUEPRINTS = $(wildcard ui/*.blp)
UIDEFS = $(addsuffix .ui,$(basename $(BLUEPRINTS)))
CATALOGS = $(wildcard po/*.po)

XGETTEXT_OPTS = \
	--package-name=$(APPID) \
	--foreign-user --copyright-holder "Sebastian Wiesner <sebastian@swsnr.de>" \
	--sort-by-file --from-code=UTF-8 --add-comments

# Extract the message template from all source files.
#
# You typically do not need to run this manually: The gettext Github workflow
# watches for changes to relevant source files, runs this target, and opens a
# pull request with the corresponding changes.
#
# When changing the set of files taken into account for xgettext also update the
# paths list in the gettext.yml workflow to make sure that updates to these
# files are caught by the gettext workflows.
#
# We strip the POT-Creation-Date from the resulting POT because xgettext bumps
# it everytime regardless if anything else changed, and this just generates
# needless diffs.
.PHONY: pot
pot:
	find src -name '*.rs' > po/POTFILES.rs
	find resources/ -name '*.blp' > po/POTFILES.blp
	xgettext $(XGETTEXT_OPTS) --language=C --keyword=dpgettext2:2c,3 --files-from=po/POTFILES.rs --output=po/de.swsnr.turnon.rs.pot
	xgettext $(XGETTEXT_OPTS) --language=C --keyword=_ --keyword=C_:1c,2 --files-from=po/POTFILES.blp --output=po/de.swsnr.turnon.blp.pot
	xgettext $(XGETTEXT_OPTS) --output=po/de.swsnr.turnon.pot \
		po/de.swsnr.turnon.rs.pot po/de.swsnr.turnon.blp.pot \
		resources/de.swsnr.turnon.metainfo.xml.in de.swsnr.turnon.desktop.in
	rm -f po/POTFILES* po/de.swsnr.turnon.rs.pot po/de.swsnr.turnon.blp.pot
	sed -i /POT-Creation-Date/d po/de.swsnr.turnon.pot

po/%.mo: po/%.po
	msgfmt --output-file $@ --check $<

# Compile binary message catalogs from message catalogs
.PHONY: msgfmt
msgfmt: $(addsuffix .mo,$(basename $(CATALOGS)))

$(LOCALEDIR)/%/LC_MESSAGES/$(APPID).mo: po/%.mo
	install -Dpm0644 $< $@

# Install compiled locale message catalogs.
.PHONY: install-locale
install-locale: $(addprefix $(LOCALEDIR)/,$(addsuffix /LC_MESSAGES/$(APPID).mo,$(notdir $(basename $(CATALOGS)))))

# Install Turn On into $DESTPREFIX using $APPID.
#
# You must run cargo build --release before invoking this target!
.PHONY: install
install: install-locale
	install -Dm0755 target/release/turnon $(DESTPREFIX)/bin/$(APPID)
	install -Dm0644 -t $(DESTPREFIX)/share/icons/hicolor/scalable/apps/ resources/icons/scalable/apps/$(APPID).svg
	install -Dm0644 resources/icons/symbolic/apps/de.swsnr.turnon-symbolic.svg \
		$(DESTPREFIX)/share/icons/hicolor/symbolic/apps/$(APPID)-symbolic.svg
	install -Dm0644 de.swsnr.turnon.desktop $(DESTPREFIX)/share/applications/$(APPID).desktop
	install -Dm0644 resources/de.swsnr.turnon.metainfo.xml  $(DESTPREFIX)/share/metainfo/$(APPID).metainfo.xml
	install -Dm0644 dbus-1/de.swsnr.turnon.service $(DESTPREFIX)/share/dbus-1/services/$(APPID).service
	install -Dm0644 de.swsnr.turnon.search-provider.ini $(DESTPREFIX)/share/gnome-shell/search-providers/$(APPID).search-provider.ini
	install -Dm0644 schemas/de.swsnr.turnon.gschema.xml $(DESTPREFIX)/share/glib-2.0/schemas/$(APPID).gschema.xml

# Patch the current git describe version into Turn On.
.PHONY: patch-git-version
patch-git-version:
	sed -Ei 's/^version = "([^"]+)"/version = "\1+$(GIT_DESCRIBE)"/' Cargo.toml
	cargo update -p turnon

# Patch the app ID to use APPID in various files
.PHONY: patch-appid
patch-appid:
	sed -i '/$(APPID)/! s/de\.swsnr\.turnon/$(APPID)/g' \
		src/config.rs \
		resources/de.swsnr.turnon.metainfo.xml.in de.swsnr.turnon.desktop.in \
		dbus-1/de.swsnr.turnon.service de.swsnr.turnon.search-provider.ini \
		schemas/de.swsnr.turnon.gschema.xml

# Remove compiled message catalogs and other generated files, and flatpak
# things
.PHONY: clean
clean:
	rm -fr po/*.mo builddir repo .flatpak-builder

# Build a development flatpak without sandbox.
.PHONY: flatpak-devel
flatpak-devel:
	flatpak run org.flatpak.Builder --force-clean --user --install \
		--install-deps-from=flathub --repo=repo \
		builddir flatpak/de.swsnr.turnon.Devel.yaml

# Build a regular flatpak (sandboxed build)
.PHONY: flatpak
flatpak:
	flatpak run org.flatpak.Builder --force-clean --sandbox --user --install \
		--install-deps-from=flathub --ccache \
		--mirror-screenshots-url=https://dl.flathub.org/media/ --repo=repo \
		builddir flatpak/de.swsnr.turnon.yaml

.PHONY: flatpak-lint-manifest
flatpak-lint-manifest:
	flatpak run --command=flatpak-builder-lint org.flatpak.Builder \
		manifest flatpak/de.swsnr.turnon.yaml

.PHONY: flatpak-lint-repo
flatpak-lint-repo:
	flatpak run --command=flatpak-builder-lint org.flatpak.Builder repo repo

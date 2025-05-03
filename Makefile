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
CATALOGS = $(wildcard po/*.po)

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
	install -Dm0644 resources/de.swsnr.turnon.metainfo.xml $(DESTPREFIX)/share/metainfo/$(APPID).metainfo.xml
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

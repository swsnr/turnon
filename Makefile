APPID = de.swsnr.turnon
BLUEPRINTS = $(wildcard ui/*.blp)
UIDEFS = $(addsuffix .ui,$(basename $(BLUEPRINTS)))
CATALOGS = $(wildcard po/*.po)
LOCALEDIR = /app/share/locale/

XGETTEXT_OPTS = \
	--package-name=$(APPID) \
	--foreign-user --copyright-holder "Sebastian Wiesner <sebastian@swsnr.de>" \
	--sort-by-file --from-code=UTF-8 --add-comments

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

.PHONY: messages
messages: pot

po/%.mo: po/%.po
	msgfmt --output-file $@ --check $<

.PHONY: msgfmt
msgfmt: $(addsuffix .mo,$(basename $(CATALOGS)))

$(LOCALEDIR)/%/LC_MESSAGES/$(APPID).mo: po/%.mo
	install -Dpm0644 $< $@

.PHONY: install-locale
install-locale: $(addprefix $(LOCALEDIR)/,$(addsuffix /LC_MESSAGES/$(APPID).mo,$(notdir $(basename $(CATALOGS)))))

.PHONY: clean
clean:
	rm -f po/*.mo

.PHONY: flatpak-devel
flatpak-devel:
	flatpak run org.flatpak.Builder --force-clean --user --install \
		--install-deps-from=flathub --repo=repo \
		builddir flatpak/de.swsnr.turnon.Devel.yaml

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

.PHONY:
devel:
	sed -i '/de.swsnr.turnon.Devel/! s/de\.swsnr\.turnon/de.swsnr.turnon.Devel/g' \
		src/config.rs \
		resources/de.swsnr.turnon.metainfo.xml.in de.swsnr.turnon.desktop.in \
		dbus-1/de.swsnr.turnon.service de.swsnr.turnon.search-provider.ini

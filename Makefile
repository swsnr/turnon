APPID = de.swsnr.turnon
BLUEPRINTS = $(wildcard ui/*.blp)
UIDEFS = $(addsuffix .ui,$(basename $(BLUEPRINTS)))
CATALOGS = $(wildcard po/*.po)
LOCALEDIR = /app/share/locale/

XGETTEXT_METADATA = \
	--package-name=$(APPID) \
	--copyright-holder "Sebastian Wiesner <sebastian@swsnr.de>"

.PHONY: pot
pot:
	find -not '(' -path '*/.*' -or -path './target/*' ')' -and \
		'(' -name '*.rs' -or \
			-name '*.desktop.in' -or \
			-name '*.metainfo.xml.in' \
			-or -name '*.blp' ')' | sort > po/POTFILES.in
	xgettext $(XGETTEXT_METADATA) --files-from=po/POTFILES.in \
		--add-comments \
		--keyword=_ --keyword=C_:1c,2 --keyword=dpgettext2:2c,3 \
		--sort-by-file --from-code=UTF-8 --output=po/de.swsnr.turnon.pot

.PHONY: messages
messages: pot $(CATALOGS)

po/*.po: po/de.swsnr.turnon.pot
	msgmerge --update --backup=none --sort-by-file --previous \
		--lang=$(notdir $(basename $@)) $@ $<

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

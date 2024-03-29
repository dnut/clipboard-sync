prefix := "/usr/local"
bin := "bin"
systemd := "lib/systemd"
profile := "release"
deb_version := $(shell grep Version control | sed 's/Version: *//g')

build:
	cargo build --profile $(profile)

install:
	install -Dm755 target/$(profile)/clipboard-sync "$(prefix)/$(bin)/clipboard-sync"
	install -Dm644 clipboard-sync.service "$(prefix)/$(systemd)/user/clipboard-sync.service"

uninstall:
	rm -f "$(prefix)/$(bin)/clipboard-sync"
	rm -f "$(prefix)/$(systemd)/user/clipboard-sync.service"

user-%: 
	$(MAKE) $* prefix="${HOME}" bin=.bin systemd=.config/systemd

deb:
	rm -rf dist/deb
	mkdir -p dist/deb/clipboard-sync_$(deb_version)/DEBIAN
	$(MAKE) install prefix=dist/deb/clipboard-sync_$(deb_version)
	cp control dist/deb/clipboard-sync_$(deb_version)/DEBIAN/control
	dpkg-deb --build dist/deb/clipboard-sync_$(deb_version)

deblint:
	lintian dist/deb/clipboard-sync_$(deb_version).deb

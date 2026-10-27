prefix := "/usr/local"
bin := "bin"
systemd := "lib/systemd"
user_unit_dir := "$(systemd)/user"
profile := "release"
version := $(shell grep '^version' Cargo.toml | sed 's/version = "\(.*\)"/\1/g')
deb_version := $(shell grep Version control | sed 's/Version: *//g')

build:
	cargo build --profile $(profile)

install:
	install -Dm755 target/$(profile)/clipboard-sync "$(prefix)/$(bin)/clipboard-sync"
	install -Dm644 clipboard-sync.service "$(prefix)/$(user_unit_dir)/clipboard-sync.service"

uninstall:
	rm -f "$(prefix)/$(bin)/clipboard-sync"
	rm -f "$(prefix)/$(user_unit_dir)/clipboard-sync.service"

user-%: 
	$(MAKE) $* prefix="${HOME}" bin=.bin systemd=.config/systemd

deb:
	rm -rf dist/deb
	mkdir -p dist/deb/clipboard-sync_$(deb_version)/DEBIAN
	$(MAKE) install prefix=dist/deb/clipboard-sync_$(deb_version)
	cp control dist/deb/clipboard-sync_$(deb_version)/DEBIAN/control
	dpkg-deb --build dist/deb/clipboard-sync_$(deb_version)

rpm:
	mkdir -p ${HOME}/rpmbuild/SOURCES
	rm -rf ${HOME}/rpmbuild/SOURCES/clipboard-sync-v$(version).tar.gz
	tar --transform 's|^|clipboard-sync-v$(version)/|' --exclude-ignore=.gitignore \
		-cf ${HOME}/rpmbuild/SOURCES/clipboard-sync-v$(version).tar.gz .
	rpmbuild -ba packaging/rpm/clipboard-sync.spec

deblint:
	lintian dist/deb/clipboard-sync_$(deb_version).deb

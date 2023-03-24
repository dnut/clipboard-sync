prefix := "/usr/local"
bin := "bin"
systemd := "lib/systemd"
profile := "release"

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

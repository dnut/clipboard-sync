prefix := "/usr/local"
bin := "bin"
systemd := "lib/systemd"

build:
	cargo build

install:
	install -Dm755 target/debug/clipboard-sync "$(prefix)/$(bin)/clipboard-sync"
	install -Dm644 clipboard-sync.service "$(prefix)/$(systemd)/user/clipboard-sync.service"

uninstall:
	rm -f "$(prefix)/$(bin)/clipboard-sync"
	rm -f "$(prefix)/$(systemd)/user/clipboard-sync.service"

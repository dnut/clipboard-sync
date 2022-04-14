import subprocess


def get(display):
    subprocess.run(["wl-paste"])


get("wayland-0")
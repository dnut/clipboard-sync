#!/bin/bash

cargo build \
    && while true; do target/debug/wayland-clipboard-sync; done \
    && (echo; echo service exited)
    || (echo; echo service failed, exiting...)

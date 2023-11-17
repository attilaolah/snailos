"""Package source details."""

load("//pkg:src_archive.bzl", "src_archive")

NAME = "busybox"
VERSION = "1.36.1"
INTEGRITY = "sha256-uMwkyVdNgJ5yecO+NJeVxdXOtv3xnKcJ+AzeUOR94xQ="
URLS = ["https://busybox.net/downloads/busybox-{v}.tar.bz2"]
NIXPKG = "os-specific/linux/busybox"

src = src_archive(
    name = NAME,
    version = VERSION,
    urls = URLS,
    integrity = INTEGRITY,
    strip_prefix = "busybox-{v}",
    patches = ["//pkg/busybox:busybox.patch"],
)

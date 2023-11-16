"""Package source details."""

load("//pkg:src_archive.bzl", "src_archive")

NAME = "z"
VERSION = "1.3"
INTEGRITY = "sha256-ipuiiY4dDXdOymultGJ6EeVYi6hciFEzbrON5GgwUKc="
URLS = ["https://www.zlib.net/zlib-{v}.tar.xz"]
NIXPKG = "development/libraries/zlib"

src = src_archive(
    name = NAME,
    version = VERSION,
    urls = URLS,
    integrity = INTEGRITY,
    strip_prefix = "zlib-{v}",
)

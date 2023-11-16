"""Package source details."""

load("//pkg:src_archive.bzl", "src_archive")

NAME = "lzo"
VERSION = "2.10"
INTEGRITY = "sha256-wPiSlDIIJm+bZUOzrjCPq2KExckOYnkxRG+0m0IhoHI="
URLS = ["https://www.oberhumer.com/opensource/lzo/download/lzo-{v}.tar.gz"]
NIXPKG = "development/libraries/lzo"

src = src_archive(
    name = NAME,
    version = VERSION,
    urls = URLS,
    integrity = INTEGRITY,
    strip_prefix = "lzo-{v}",
)

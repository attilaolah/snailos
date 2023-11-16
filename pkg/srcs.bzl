"""All package sources."""

load("//pkg/lzo:src.bzl", lzo_src = "src")
load("//pkg/z:src.bzl", z_src = "src")

def srcs():
    lzo_src()
    z_src()

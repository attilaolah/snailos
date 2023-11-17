"""All package sources."""

load("//pkg/busybox:src.bzl", busybox_src = "src")
load("//pkg/lzo:src.bzl", lzo_src = "src")
load("//pkg/z:src.bzl", z_src = "src")

def srcs():
    busybox_src()
    lzo_src()
    z_src()

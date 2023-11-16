"""Macro for creating a source archive."""

load("//:http_archive.bzl", "http_archive")

def src_archive(name, **kwargs):
    kwargs["name"] = "{}_src".format(name)
    kwargs.setdefault("filegroups", {}).setdefault("all", 'glob(["**"])')
    return lambda: http_archive(**kwargs)

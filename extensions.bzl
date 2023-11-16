"""Registers any non-module dependencies."""

load("//pkg:srcs.bzl", "srcs")

def _non_module_dependencies_impl(_ctx):
    srcs()

non_module_dependencies = module_extension(
    implementation = _non_module_dependencies_impl,
)

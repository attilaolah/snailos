"""Wrappers for @rules_foreign_cc for using the Emscripten toolchain."""

load("@rules_foreign_cc//foreign_cc:cmake.bzl", _cmake = "cmake")
load("@rules_foreign_cc//foreign_cc:configure.bzl", _configure_make = "configure_make")
load("@rules_foreign_cc//foreign_cc:make.bzl", _make = "make")

EM_TOOLS = [
    "@emscripten_bin_linux//:all",
    "@emsdk//emscripten_toolchain:env.sh",
    "@nodejs//:node",
]

EM_ENV = {
    "EMENV": "$(execpath @emsdk//emscripten_toolchain:env.sh)",
    "CROSSCOMPILING_EMULATOR": "$(execpath @nodejs//:node)",
}

EM_PREFIX = " && ".join([
    "source $${EMENV}",
    "EM_PKG_CONFIG_PATH=$${PKG_CONFIG_PATH:-}",
])

EM_HOST_CONSTRAINTS = [
    "@platforms//os:linux",
    "@platforms//cpu:x86_64",
]

EM_CACHE_ENTRIES = {
    "CMAKE_CROSSCOMPILING_EMULATOR": "${CROSSCOMPILING_EMULATOR}",
    "CMAKE_MODULE_PATH": "${EMSCRIPTEN}/cmake/Modules",
    "CMAKE_SYSTEM_NAME": "Emscripten",
    "CMAKE_TOOLCHAIN_FILE": "${EMSCRIPTEN}/cmake/Modules/Platform/Emscripten.cmake",
}

def cmake(name, **kwargs):
    """Wrapper around cmake() from @rules_foreign_cc.

    Args:
      name: Package name.
      **kwargs: Updated and passed on to @rules_foreign_cc.
    """
    _defaults(name, kwargs)
    kwargs.setdefault("tool_prefix", _select(wasm = _em_tool("cmake")))
    kwargs.setdefault("cache_entries", _select(wasm = EM_CACHE_ENTRIES, default = {}))

    _cmake(name = name, **kwargs)

def configure_make(name, **kwargs):
    """Wrapper around configure_make() from @rules_foreign_cc.

    Args:
      name: Package name.
      **kwargs: Updated and passed on to @rules_foreign_cc.
    """
    _defaults(name, kwargs)
    kwargs.setdefault("configure_prefix", _select(wasm = _em_tool("configure")))
    kwargs.setdefault("tool_prefix", _select(wasm = _em_tool("make")))

    _configure_make(name = name, **kwargs)

def make(name, **kwargs):
    """Wrapper around make() from @rules_foreign_cc.

    Args:
      name: Package name.
      **kwargs: Updated and passed on to @rules_foreign_cc.
    """
    _defaults(name, kwargs)
    kwargs.setdefault("tool_prefix", _select(wasm = _em_tool("make")))

    _make(name = name, **kwargs)

def keyval(*args):
    """Convenience macro for generating key=val pairs."""
    kwargs = {}
    [kwargs.update(arg) for arg in args]
    return ["{}={}".format(key, val) for key, val in kwargs.items()]

def _defaults(name, kwargs):
    basename = name.removesuffix(".pkg")
    kwargs.setdefault("lib_source", "@{}_src//:all".format(basename))
    if "out_binaries" not in kwargs:
        kwargs.setdefault("out_static_libs", ["lib{}.a".format(basename)])

    kwargs["build_data"] = _select(
        wasm = kwargs.get("build_data", []) + EM_TOOLS,
        default = kwargs.get("build_data", []),
    )

    kwargs["env"] = _select(
        wasm = dict(EM_ENV.items() + kwargs.get("env", {}).items()),
        default = kwargs.get("env", {}),
    )

    # Currently we hardcode @emscripten_bin_linux.
    # This means the execution platform must support these binaries.
    kwargs["exec_compatible_with"] = EM_HOST_CONSTRAINTS

def _em_tool(tool):
    return "{} $${{EMSCRIPTEN}}/em{}".format(EM_PREFIX, tool)

def _select(wasm, default = None):
    """Convenience wrapper around select()."""
    return select({
        "@platforms//cpu:wasm32": wasm,
        "//conditions:default": default,
    })

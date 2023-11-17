"""Wrappers for @rules_foreign_cc for using the Emscripten toolchain."""

load("@rules_foreign_cc//foreign_cc:configure.bzl", _configure_make = "configure_make")

EM_TOOLS = [
    "@emscripten_bin_linux//:all",
    "@emsdk//emscripten_toolchain:env.sh",
]

EM_ENV = {
  "EMENV": "$(execpath @emsdk//emscripten_toolchain:env.sh)",
}

EM_PREFIX = " && ".join([
    "source $${EMENV}",
    "EM_PKG_CONFIG_PATH=$${PKG_CONFIG_PATH:-}",
])

def configure_make(name, **kwargs):
    """Wrapper around configure_make from @rules_foreign_cc.

    Args:
      name: Package name.
      **kwargs: Updated and passed on to @rules_foreign_cc.
    """
    kwargs.setdefault("lib_source", "@{}_src//:all".format(name))
    kwargs.setdefault("out_static_libs", ["lib{}.a".format(name)])

    # Emscripten tools:
    kwargs["build_data"] = _select(
        wasm = kwargs.get("build_data", []) + EM_TOOLS,
        default = kwargs.get("build_data", []),
    )
    kwargs.setdefault("configure_prefix", _select(wasm = _em_tool("configure")))
    kwargs.setdefault("tool_prefix", _select(wasm = _em_tool("make")))

    env = kwargs.get("env", {})
    kwargs["env"] = _select(
      wasm = dict(EM_ENV.items() + env.items()),
      default = env,
    )

    # Currently we hardcode @emscripten_bin_linux.
    # This means the execution platform must support these binaries.
    kwargs["exec_compatible_with"] = [
        "@platforms//os:linux",
        "@platforms//cpu:x86_64",
    ]

    _configure_make(name = name, **kwargs)

def _select(wasm, default = None):
    """Convenience wrapper around select()."""
    return select({
        "@platforms//cpu:wasm32": wasm,
        "//conditions:default": default,
    })

def _em_tool(tool):
    return "{} $${{EMSCRIPTEN}}/em{}".format(EM_PREFIX, tool)

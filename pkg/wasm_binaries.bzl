def _wasm_transition_impl(settings, attr):
    return {"//command_line_option:platforms": "@emsdk//:platform_wasm"}

wasm_transition = transition(
    implementation = _wasm_transition_impl,
    inputs = [],
    outputs = ["//command_line_option:platforms"],
)

def _wasm_binaries_impl(ctx):
    inputs = []
    for src in ctx.files.srcs:
        if src.dirname.rpartition("/")[-1] != "bin":
            continue
        if not (src.basename.endswith(".js") or src.basename.endswith(".wasm")):
            continue
        if len(src.dirname.partition("/bin/pkg/")[-1].split("/")) != 3:
            continue  # expected: pkg_name/rule_name/bin
        inputs.append(src)

    outputs = []
    for file in inputs:
        output = ctx.actions.declare_file(
            "{}/{}".format(ctx.attr.name, file.basename),
        )
        ctx.actions.symlink(output = output, target_file = file)
        outputs.append(output)

    return DefaultInfo(files = depset(outputs))

wasm_binaries = rule(
    implementation = _wasm_binaries_impl,
    attrs = {
        "srcs": attr.label_list(
            doc = "Sources that generate JS + Wasm binary outputs.",
            cfg = wasm_transition,
        ),
        "_allowlist_function_transition": attr.label(
            default = "@bazel_tools//tools/allowlists/function_transition_allowlist",
        ),
    },
)

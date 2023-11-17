"""Rule for generating a BusyBox config."""

SHELL_CMD = r"""
IFS=';' read -ra sources <<< "${SRCS}"
cat $(
  # Find regular files and symlinks pointing to files,
  # BUT, exclude symlinks pointing to directories. Thank you, ChatGPT.
  find ${sources} -type f -exec echo {} \; -o -type l -exec test ! -d {} \; -print
) > ${OUT}

IFS=';' read -ra removes <<< "${REMOVE}"
for config in "${removes[@]}"; do
  echo REM: $config
  sed -i "s/^${config}=.*/# ${config} is not set/" ${OUT}
done

IFS=';' read -ra replacements <<< "${REPLACE}"
for replacement in "${replacements[@]}"; do
  config="${replacement%=*}"
  value="${replacement#*=}"
  echo REPL: $config $value

  sed -i "s/^# ${config} is not set$/# ${config} is not set/; s/^# ${config} is not set/${config}=${value}/" ${OUT}
done
"""

def _busybox_config_impl(ctx):
    if ctx.attr.base == None and len(ctx.attr.remove) > 0:
        fail("Cannot have 'remove' if 'base' was not provided.")

    output = ctx.actions.declare_file("{}.conf".format(ctx.attr.name))

    ctx.actions.run_shell(
        command = SHELL_CMD,
        inputs = ctx.files.base,
        outputs = [output],
        env = {
            "OUT": output.path,
            "INPUTS": ";".join([file.path for file in ctx.files.base]),
            "REMOVE": ";".join([
                "CONFIG_{}".format(config.upper())
                for config in ctx.attr.remove
            ]),
            "REPLACE": ";".join([
                "CONFIG_{}={}".format(config.upper(), value)
                for config, value in ctx.attr.values.items()
            ]),
        },
    )
    return [DefaultInfo(files = depset([output]))]

busybox_config = rule(
    implementation = _busybox_config_impl,
    attrs = {
        "base": attr.label(
            doc = "Base config to modify. An empty file will be used if omitted.",
            mandatory = False,
        ),
        "values": attr.string_dict(
            doc = "Values to add to the config. Values will be converted to upper-case.",
            mandatory = False,
            default = {},
        ),
        "remove": attr.string_list(
            doc = "List of values to remove from the base config.",
            mandatory = False,
            default = [],
        ),
    },
)

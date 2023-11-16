"""Custom http_archive() macro.

Replaces version numbers in URLs and path prefixes to avoid having to hard-code
the version. This allows the version number to be specified in a central place.

Inspired by Gentoo's package configuration files.
"""

load("@bazel_tools//tools/build_defs/repo:http.bzl", _http_archive = "http_archive")

FILEGROUP = """\
filegroup(
    name = "{name}",
    srcs = {srcs},
    visibility = ["//visibility:public"],
)
"""

def http_archive(
        version,
        urls,
        strip_prefix = None,
        filegroups = None,
        **kwargs):
    """Wrapper around http_archive().

    It specifies a common BUILD file by default, which publically exports all
    contents.

    Additionally, it allows template substitution in the URLs and in
    strip_prefix.

    Args:
      version: Used only for template substitution in the urls and strip_prefix
        params.
      urls: Passed on to http_archive() after template substitution.
      strip_prefix: Passed on to http_archive() after template substitution.
      filegroups: Additional filegroups to add to the BUILD file. This should
        be a dict, e.g. {"all": 'glob(["**"])'}.
      **kwargs: Passed on to http_archive().

    """
    args = {
        "v": version,  # 1.2.3
        "v-": version.replace(".", "-"),  # 1-2-3
        "v_": version.replace(".", "_"),  # 1_2_3
        "vm": version.split(".")[0],  # 1
        "vmm": "".join(version.split(".")[:2]),  # 12
        "vmmd": ".".join(version.split(".")[:2]),  # 1.2
    }

    if strip_prefix != None:
        strip_prefix = strip_prefix.format(**args)

    if filegroups != None:
        build_file_content = ""
        for name, srcs in (filegroups or {}).items():
            if type(srcs) != str:
                srcs = str(srcs)
            build_file_content += FILEGROUP.format(name = name, srcs = srcs)
        kwargs["build_file_content"] = build_file_content

    _http_archive(
        urls = [url.format(**args) for url in urls],
        strip_prefix = strip_prefix,
        **kwargs
    )

"""Repository rules for downloading all dependencies."""

load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

def workspace_dependencies():
    """Set up dependencies of THIS workspace."""
    http_archive(
        name = "rules_rust",
        sha256 = "6357de5982dd32526e02278221bb8d6aa45717ba9bbacf43686b130aa2c72e1e",
        urls = ["https://github.com/bazelbuild/rules_rust/releases/download/0.30.0/rules_rust-v0.30.0.tar.gz"],
    )

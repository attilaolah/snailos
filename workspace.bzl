"""Repository rules for downloading all dependencies."""

load(":http_archive.bzl", "http_archive")

def workspace_dependencies():
    """Set up dependencies of THIS workspace."""
    http_archive(
        name = "emsdk",
        version = "3.1.49",
        urls = ["https://github.com/emscripten-core/emsdk/archive/refs/tags/{v}.tar.gz"],
        integrity = "sha256-yZ2Y2pJBx+cnhLx2Sj5gpl2PJyAtRfPNQisqxyRTgNk=",
        strip_prefix = "emsdk-{v}/bazel",
    )
    http_archive(
        name = "rules_rust",
        version = "0.30.0",
        integrity = "sha256-Y1feWYLdMlJuAieCIbuNaqRXF7qbus9DaGsTCqLHLh4=",
        urls = ["https://github.com/bazelbuild/rules_rust/releases/download/{v}/rules_rust-v{v}.tar.gz"],
    )

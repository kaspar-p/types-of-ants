load("@io_bazel_rules_go//go:def.bzl", "cli_binary")

cli_binary(
    name = "add",
    srcs = ["cli/main.go"],
    embed = [":go_default_library"],
    visibility = ["//visibility:public"],
)

load("@rules_rust//rust:defs.bzl", "rust_binary")

rust_binary(
    name = "ant-just-checking-in",
    srcs = [
        "src/emit.rs",
        "src/main.rs",
    ],
    edition = "2021",
    deps = [
        "//ant-just-checking-in/cargo:cronjob",
        "//ant-just-checking-in/cargo:postgres",
        "//ant-just-checking-in/cargo:reqwest",
    ],
)

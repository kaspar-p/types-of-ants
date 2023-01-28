load("@rules_rust//rust:defs.bzl", "rust_binary")

rust_binary(
    name = "ant-just-checking-in",
    srcs = [
        "src/db.rs",
        "src/main.rs",
        "src/test.rs",
        "src/tests/mod.rs",
        "src/tests/ping.rs",
    ],
    edition = "2021",
    deps = [
        "//ant-just-checking-in/cargo:chrono",
        "//ant-just-checking-in/cargo:cronjob",
        "//ant-just-checking-in/cargo:postgres",
        "//ant-just-checking-in/cargo:r2d2",
        "//ant-just-checking-in/cargo:r2d2_postgres",
        "//ant-just-checking-in/cargo:reqwest",
        # "//ant-just-checking-in/cargo:uuid",
    ],
)

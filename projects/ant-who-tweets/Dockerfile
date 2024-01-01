FROM clux/muslrust:stable AS chef
USER root
RUN cargo install cargo-chef
WORKDIR /types-of-ants

FROM chef AS planner
COPY ./Cargo.lock ./Cargo.toml ./
COPY ./projects ./projects
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /types-of-ants/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
COPY ./Cargo.lock ./Cargo.toml ./
COPY ./projects ./projects
RUN cargo build --release --target x86_64-unknown-linux-musl --bin ant-who-tweets

FROM alpine AS runtime
WORKDIR /types-of-ants
COPY --from=builder /types-of-ants/target/x86_64-unknown-linux-musl/release/ant-who-tweets /usr/local/bin
USER root
CMD ["/usr/local/bin/ant-who-tweets"]

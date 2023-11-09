FROM rust:1.66.0 AS build
WORKDIR /types-of-ants

RUN rustup target add x86_64-unknown-linux-musl

# Build package cache for ant-data-farm
RUN cargo new ant-data-farm
WORKDIR /types-of-ants/ant-data-farm
COPY ./projects/ant-data-farm/Cargo.toml ../ant-data-farm/Cargo.lock ./
RUN cargo build --release

# Build package cache for ant-building-projects
RUN cargo new ant-building-projects


RUN cargo new ant-gateway
RUN cargo new ant-host-agent
RUN cargo new ant-just-checking-in
RUN cargo new ant-metadata
RUN cargo new ant-on-the-web
RUN cargo new ant-owning-artifacts
RUN cargo new ant-who-tweets
RUN cargo new anthill

COPY 

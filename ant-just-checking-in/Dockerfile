###########################
#    The builder image    #
###########################
FROM rust:latest AS builder

RUN update-ca-certificates
ENV USER=typesofants
ENV UID=10001

RUN adduser \
  --disabled-password \
  --gecos "" \
  --home "/nonexistant" \
  --shell "/sbin/nologin" \
  --no-create-home \
  --uid "${UID}" \
  "${USER}"

WORKDIR /ant-just-checking-in
COPY ./ .
RUN cargo build --release


#############################
#    The execution image    #
#############################
FROM debian:buster-slim
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /ant-just-checking-in

COPY --from=builder /ant-just-checking-in/target/release/ant-just-checking-in ./
USER typesofants:typesofants

CMD "/ant-just-checking-in"

ARG TARGET
ARG ARCH

FROM --platform=${TARGET} debian:bullseye-slim
WORKDIR /app

ARG ARCH

RUN apt-get -y update; apt-get -y install curl

RUN touch .env
COPY build/release/${ARCH}/ant-archive-storage ant-archive-storage

CMD ["./ant-archive-storage"]

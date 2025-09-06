ARG TARGET

FROM --platform=${TARGET} debian:bullseye-slim
WORKDIR /app

RUN apt-get -y update; apt-get -y install curl

RUN mkdir -p fs
COPY build/release/ant-fs ant-fs
COPY --from=root secrets/prod/build.cfg .env

CMD "./ant-fs"

# build container

FROM rust:1.32 as rust-build
LABEL maintainer "Devolutions Inc."

WORKDIR /opt/qui-vive

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src

RUN cargo build --release

# production container

FROM debian:stretch-slim
LABEL maintainer "Devolutions Inc."

WORKDIR /opt/qui-vive

RUN apt-get update
RUN apt-get install -y --no-install-recommends libssl1.1 ca-certificates
RUN rm -rf /var/lib/apt/lists/*

COPY --from=rust-build /opt/qui-vive/target/release/qui-vive .

EXPOSE 8080

ENTRYPOINT [ "./qui-vive" ]

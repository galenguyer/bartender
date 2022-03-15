FROM docker.io/rust:1.59.0-bullseye AS builder

WORKDIR /src/bartender/
RUN cargo init --bin
COPY Cargo.toml Cargo.lock .
RUN cargo build --release

COPY . .
RUN cargo build --release

FROM docker.io/debian:bullseye-slim
RUN apt update -qy && apt upgrade -qy && apt install -qy ca-certificates

COPY --from=builder /src/bartender/target/release/bartender /bartender
ENTRYPOINT ["/bartender"]

FROM rustlang/rust:nightly-slim AS builder
WORKDIR /usr/src/app

RUN apt-get update && apt-get install -y pkg-config libssl-dev default-libmysqlclient-dev && rm -rf /var/lib/apt/lists/*

COPY . .
RUN cargo install --path .

FROM debian:stable-slim
RUN apt-get update && apt-get install -y ca-certificates openssl libmariadb3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/avrae-plus /usr/local/bin/avrae-plus

CMD ["avrae-plus"]
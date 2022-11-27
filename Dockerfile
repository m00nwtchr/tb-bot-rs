FROM rustlang/rust:nightly-slim AS builder
WORKDIR /usr/src/app

RUN apt-get update && apt-get install -y pkg-config libssl-dev default-libmysqlclient-dev && rm -rf /var/lib/apt/lists/*

COPY . .
RUN --mount=type=cache,target=/usr/src/app/target cargo install --path .

FROM debian:buster-slim

RUN apt-get update && apt-get install -y openssl libmariadb3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/avrae-plus /usr/local/bin/avrae-plus

CMD ["avrae-plus"]
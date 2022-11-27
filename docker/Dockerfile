FROM rust:latest
WORKDIR /usr/src/app

COPY . .
RUN cargo install --path .

#FROM debian:buster-slim
#RUN apt-get update && rm -rf /var/lib/apt/lists/*
#COPY --from=builder /usr/local/cargo/bin/avrae-plus /usr/local/bin/avrae-plus

CMD ["avrae-plus"]
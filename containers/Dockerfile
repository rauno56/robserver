FROM rust:1.75-bookworm as builder
WORKDIR /usr/src/app

COPY . .
RUN cargo install --locked --path .
RUN ls -lah /usr/local/cargo/bin


FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y openssl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/robserver /usr/local/bin/robserver

ENTRYPOINT ["sh", "-c"]
CMD ["robserver"]

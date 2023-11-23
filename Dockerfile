FROM rust:1.73-bookworm as builder
WORKDIR /usr/src/app

COPY . .
RUN cargo install --locked --path .
RUN ls -lah /usr/local/cargo/bin


FROM builder as migrations
RUN cargo install sqlx-cli --no-default-features --features native-tls,postgres


FROM debian:bookworm-slim
# RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/robserver /usr/local/bin/robserver

ENTRYPOINT ["sh", "-c"]
CMD ["robserver"]

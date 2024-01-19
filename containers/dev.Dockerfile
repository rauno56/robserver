FROM rust:1.75-bookworm as builder
WORKDIR /usr/src/app

RUN cargo install cargo-watch

COPY . .
RUN cargo build

CMD ["cargo", "watch", "-x", "run"]

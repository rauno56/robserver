FROM rust:1.75-bookworm as builder
WORKDIR /usr/src/app

RUN cargo install sqlx-cli --no-default-features --features native-tls,postgres
RUN rm -rf $CARGO_HOME/registry

COPY migrations migrations
COPY scripts/sqlx-create-run sqlx-create-run

RUN chmod +x sqlx-create-run
CMD ["bash", "sqlx-create-run"]

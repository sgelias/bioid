# ? ----------------------------------------------------------------------------
# ? Build stage/
# ? ----------------------------------------------------------------------------

FROM rust:latest as builder

WORKDIR /rust

# ? The copy operations are performed in sepparate steps to allow caching layers
# ? over building operations
COPY core /rust/core
COPY adapters /rust/adapters
COPY ports /rust/ports
COPY test /rust/test
COPY Cargo.toml /rust/Cargo.toml
COPY Cargo.lock /rust/Cargo.lock

RUN cargo build --bin myc-api --release

# ? ----------------------------------------------------------------------------
# ? Production stage
# ? ----------------------------------------------------------------------------

FROM rust:latest

COPY --from=builder /rust/target/release/myc-api /usr/local/bin/myc-api

ARG SERVICE_PORT=8080
ENV SERVICE_PORT=${SERVICE_PORT}

EXPOSE ${SERVICE_PORT}

ENTRYPOINT ["myc-api"]

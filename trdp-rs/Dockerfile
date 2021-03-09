FROM rust:buster

RUN apt-get update && apt-get install -y --no-install-recommends \
        llvm-dev libclang-dev clang uuid-dev

WORKDIR /trdp

# Cache dependencies
# RUN mkdir src && touch src/lib.rs
# COPY ./Cargo.toml .
# RUN cargo build --release

COPY --from=reg.osmhpi.de/railchain/tcnopen-trdp:latest /usr/local/include/trdp /usr/local/include/trdp
COPY --from=reg.osmhpi.de/railchain/tcnopen-trdp:latest /usr/local/lib/libtrdpap.a /usr/local/lib/

COPY . ./
# RUN cargo build --release
RUN cargo test

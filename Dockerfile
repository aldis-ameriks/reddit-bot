FROM ekidd/rust-musl-builder as builder

WORKDIR /home/rust/

# Avoid having to install/build all dependencies by copying
# the Cargo files and making a dummy src/main.rs
COPY Cargo.toml .
COPY Cargo.lock .
RUN echo "fn main() {}" > src/main.rs
RUN cargo test
RUN cargo build --release

# We need to touch our real main.rs file or else docker will use
# the cached one.
COPY . .
RUN sudo touch src/main.rs

RUN cargo test
RUN cargo build --release

# Size optimization
RUN strip target/x86_64-unknown-linux-musl/release/reddit-bot

FROM alpine:latest
RUN apk --no-cache add ca-certificates
COPY --from=builder /home/rust/target/x86_64-unknown-linux-musl/release/reddit-bot .
ENTRYPOINT ["./reddit-bot"]

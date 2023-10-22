FROM rust:slim-bookworm as builder

WORKDIR /usr/src/app
COPY . .
# Build and cache the binary and dependent crates in release mode
RUN --mount=type=cache,target=/usr/local/cargo,from=rust:latest,source=/usr/local/cargo \
    --mount=type=cache,target=target \
    cargo build --release && mv ./target/release/signaling ./signaling

# Runtime image
FROM debian:bookworm-slim

# Run as "app" user
RUN useradd -ms /bin/bash app

# Chown data directory
RUN mkdir /usr/local/share/signaling && chown app:app /usr/local/share/signaling

USER app
WORKDIR /app

# Get compiled binaries from builder's cargo install directory
COPY --from=builder /usr/src/app/signaling /app/signaling

# Run the app
CMD ./signaling

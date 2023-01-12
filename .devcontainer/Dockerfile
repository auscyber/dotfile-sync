FROM mcr.microsoft.com/devcontainers/rust:latest
RUN rustup toolchain install nightly
RUN rustup component add clippy --toolchain "nightly"
RUN cargo install cargo-edit cargo-expand
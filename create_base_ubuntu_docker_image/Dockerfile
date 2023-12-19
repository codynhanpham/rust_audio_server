# Start with Ubuntu Trusty as base image
FROM ubuntu:trusty

# Update the package lists
RUN apt-get update

# Install curl
RUN apt-get install -y curl build-essential pkg-config

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Add Rust to PATH
ENV PATH="/root/.cargo/bin:${PATH}"

# Install alsa and alsa-tools
RUN apt-get install -y alsa alsa-tools

# Install libasound2-dev
RUN apt-get install -y libasound2-dev
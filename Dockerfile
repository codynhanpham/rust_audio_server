# Start with Ubuntu Trusty as base image
FROM ubuntu:trusty

# Update the package lists
RUN apt-get update

# Install curl
RUN apt-get install -y curl

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Add Rust to PATH
ENV PATH="/root/.cargo/bin:${PATH}"

# Install alsa and alsa-tools
RUN apt-get install -y alsa alsa-tools

# Install libasound2-dev
RUN apt-get install -y libasound2-dev

# Copy the current directory contents into the Docker image
COPY . .

# Build the Rust application
RUN cargo build --release

# Copy the compiled executable to the current directory
RUN cp target/release/* ./ubuntu_release
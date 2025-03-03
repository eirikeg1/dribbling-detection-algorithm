FROM rust:1.85-slim-bullseye

# Install additional packages
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    pkg-config \
    libopencv-dev \
    libssl-dev \
    clang \
    libclang-dev \
    llvm-dev \
    git \
    curl \
    unzip \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Copy your project files
COPY . .

# Install cargo-watch (cargo is already available in the PATH)
RUN cargo install cargo-watch

# Build your project (optional)
RUN cargo build --release

# Copy entrypoint scripts and set permissions
COPY update_env.sh /app/update_env.sh
COPY entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/update_env.sh /app/entrypoint.sh

# Set the entrypoint to your script
ENTRYPOINT ["/app/entrypoint.sh"]

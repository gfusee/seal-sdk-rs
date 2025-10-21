FROM debian:bookworm-slim AS runner

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder
COPY --from=seal-sdk-rs-sui-bin /usr/local/bin/sui /usr/local/bin/sui

ENTRYPOINT ["sui", "start", "--with-faucet", "--force-regenesis"]


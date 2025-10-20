FROM debian:bookworm-slim AS runner

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    curl \
    jq \
    git \
    && rm -rf /var/lib/apt/lists/*

COPY --from=sui-binary /usr/local/bin/sui /usr/local/bin/sui
COPY --from=seal-binary /seal /seal
COPY --from=seal-binary /usr/local/bin/seal-cli /usr/local/bin/seal-cli
COPY --from=seal-binary /usr/local/bin/key-server /usr/local/bin/key-server
COPY run.sh run.sh
COPY wait_for_localnet.sh wait_for_localnet.sh
COPY empty_sui_config.yaml /root/.sui/sui_config/client.yaml
COPY faucet.sh faucet.sh
COPY deploy_seal.sh deploy_seal.sh

RUN chmod +x run.sh
RUN chmod +x wait_for_localnet.sh
RUN chmod +x faucet.sh
RUN chmod +x deploy_seal.sh

ENTRYPOINT ["./run.sh", "http://localnet:9000"]
# ENTRYPOINT ["/bin/bash"]

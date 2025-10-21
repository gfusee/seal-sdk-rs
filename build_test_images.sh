#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "$0")" && pwd)
BIN_DIR="$ROOT_DIR/utils/docker/bins"
LOCALNET_DIR="$ROOT_DIR/utils/docker/localnet"
SEAL_SERVER_DIR="$ROOT_DIR/utils/docker/seal-server"

log() {
  echo "[build-test-images] $1"
}

log "Building Sui binary image (seal-sdk-rs-sui-bin)"
docker build -f "$BIN_DIR/Dockerfile.sui" -t seal-sdk-rs-sui-bin "$BIN_DIR"

log "Building Seal binary image (seal-sdk-rs-seal-bin)"
docker build -f "$BIN_DIR/Dockerfile.seal" -t seal-sdk-rs-seal-bin "$BIN_DIR"

log "Building localnet image (seal-sdk-rs-localnet:latest)"
docker build -f "$LOCALNET_DIR/Dockerfile" -t seal-sdk-rs-localnet:latest "$LOCALNET_DIR"

log "Building seal server image (seal-sdk-rs-seal-server:latest)"
docker build -f "$SEAL_SERVER_DIR/Dockerfile" -t seal-sdk-rs-seal-server:latest "$SEAL_SERVER_DIR"

log "Done."

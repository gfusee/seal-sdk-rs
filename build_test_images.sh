#!/usr/bin/env bash
# Copyright 2025 Quentin Diebold
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

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

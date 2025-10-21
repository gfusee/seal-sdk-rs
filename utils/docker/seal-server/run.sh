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

to_vector() {
    local hex=${1#0x}
    local -a parts=()
    [[ $(( ${#hex} % 2 )) -ne 0 ]] && hex="0$hex"
    for ((i=0; i<${#hex}; i+=2)); do
        parts+=("0x${hex:i:2}")
    done
    echo "vector[${parts[*]}]" | sed 's/ /, /g'
}

NODE_URL="${NODE_URL:-http://localnet:9000}"
echo "Using node URL: $NODE_URL"

FAUCET_URL="${FAUCET_URL:-http://localnet:9123}"
echo "Using faucet URL: $FAUCET_URL"

./wait_for_localnet.sh $NODE_URL

sui client new-address ed25519
sui client new-env --alias localnet --rpc $NODE_URL
./faucet.sh "$FAUCET_URL" "$(sui client active-address)"

SEAL_PACKAGE_ID="$(./deploy_seal.sh)"

echo "Seal package id: $SEAL_PACKAGE_ID"

# Generate keys
OUTPUT=$(seal-cli genkey)

# Print the full output
echo "$OUTPUT"

# Extract the Master key value
MASTER_KEY=$(echo "$OUTPUT" | grep -oP '(?<=Master key:\s)0x[0-9a-fA-F]+')
PUBLIC_KEY=$(echo "$OUTPUT" | grep -oP '(?<=Public key:\s)0x[0-9a-fA-F]+')

# Use SEAL_SERVER_URL if defined, otherwise default to localhost:9005
SEAL_SERVER_URL="${SEAL_SERVER_URL:-http://localhost:9005}"
echo "Using Seal server URL: $SEAL_SERVER_URL"

KEY_SERVER_JSON="$(sui client ptb \
  --move-call $SEAL_PACKAGE_ID::key_server::create_and_transfer_v1 '"FirstServer"' "\"$SEAL_SERVER_URL\"" 0 $(to_vector "$PUBLIC_KEY") \
  --json
)"

KEY_SERVER_OBJECT_ID="$(
  jq -r '.objectChanges[]
         | select(.type=="created" and (.objectType | endswith("::key_server::KeyServer")))
         | .objectId' \
    <<< "$KEY_SERVER_JSON"
)"

echo "Seal package ID: $SEAL_PACKAGE_ID"
echo "Key server ID: $KEY_SERVER_OBJECT_ID"
echo "Master key: $MASTER_KEY"

# Ensure the /shared directory exists
mkdir -p /shared

# Write the variables into /shared/seal.json
cat <<EOF > /shared/seal.json
{
  "seal_package_id": "$SEAL_PACKAGE_ID",
  "key_server_object_id": "$KEY_SERVER_OBJECT_ID",
  "public_key": "$PUBLIC_KEY"
}
EOF

# Run the key-server command
MASTER_KEY="$MASTER_KEY" NETWORK="custom" NODE_URL="$NODE_URL" KEY_SERVER_OBJECT_ID="$KEY_SERVER_OBJECT_ID" key-server

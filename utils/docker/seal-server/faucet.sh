#!/usr/bin/env bash

FAUCET_URL="$1"
HEX_ADDRESS="$2"

if [ -z "$FAUCET_URL" ] || [ -z "$HEX_ADDRESS" ]; then
  echo "Usage: $0 <faucet_url> <hex_address>"
  exit 1
fi

curl -X POST "$FAUCET_URL/v2/gas" \
  -H "Content-Type: application/json" \
  -d "{\"FixedAmountRequest\":{\"recipient\":\"$HEX_ADDRESS\"}}"

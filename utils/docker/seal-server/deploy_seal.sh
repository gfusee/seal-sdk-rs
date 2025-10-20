publish_json() {
  local dir="$1"
  log "+ (cd $dir && sui client publish --json)"
  ( cd "$dir" && sui client publish --json )
}

SEAL_JSON="$(publish_json "/seal/move/seal")"

PACKAGE_ID="$(
  jq -r '[.objectChanges[] | select(.type=="published")] | last | .packageId' \
    <<< "$SEAL_JSON"
)"

echo "$PACKAGE_ID"

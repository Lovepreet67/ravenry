#!/usr/bin/env bash
set -Eeuo pipefail

# Configuration (can be overridden through environment variables)
HOST="${HOST:-http://localhost:8080}"
TENANT_ID="${TENANT_ID:-15cb0012-111e-427c-8c53-37f26cfb5f9d}"
CONNECTIONS="${CONNECTIONS:-10}"
DURATION="${DURATION:-30}"
PIPELINING="${PIPELINING:-1}"

# Fail if AUTH_TOKEN is missing or empty
if [[ -z "${AUTH_TOKEN:-}" ]]; then
    echo "Error: AUTH_TOKEN is not set or is empty." >&2
    echo "Set it with: export AUTH_TOKEN='your-token'" >&2
    exit 1
fi

# Fail if autocannon is not installed or not on PATH
if ! command -v autocannon >/dev/null 2>&1; then
    echo "Error: autocannon is not installed or not on PATH." >&2
    echo "Install it with: npm install -g autocannon" >&2
    exit 1
fi

# Validate numeric configuration
for name in CONNECTIONS DURATION PIPELINING; do
    value="${!name}"

    if [[ ! "$value" =~ ^[0-9]+$ ]] || (( value < 1 )); then
        echo "Error: $name must be a positive integer (got '$value')." >&2
        exit 1
    fi
done

URL="${HOST%/}/tenants/${TENANT_ID}/events"

echo "Starting Autocannon load test"
echo "Target:      $URL"
echo "Connections: $CONNECTIONS"
echo "Duration:    ${DURATION}s"
echo "Pipelining:  $PIPELINING"
echo

exec autocannon 
    -m POST 
    -c "$CONNECTIONS" 
    -d "$DURATION" 
    -p "$PIPELINING" 
    -H 'Content-Type=application/json' 
    -H "Authorization=Bearer ${AUTH_TOKEN}" 
    -b '{"topic_path":"test.testing","payload":{"name":"lovepreet"}}' 
    --renderStatusCodes 
    "$URL"
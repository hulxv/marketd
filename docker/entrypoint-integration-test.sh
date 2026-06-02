#!/bin/sh
set -e

# Start nostr-rs-relay in the background. The current e2e test passes
# `nostr_relays: vec![]` so the relay is not strictly required, but starting
# it keeps parity with maker-dashboard and lets us add maker-spawning tests
# without touching this script.
/root/.nostr-relay-bin/nostr-rs-relay \
    --config /etc/nostr-relay/config.toml \
    > /tmp/nostr-relay.log 2>&1 &

echo "[entrypoint] Waiting for nostr relay on 127.0.0.1:8000 ..."
i=0
while ! nc -z 127.0.0.1 8000; do
    i=$((i + 1))
    if [ "$i" -ge 40 ]; then
        echo "[entrypoint] Relay failed to start. Logs:"
        cat /tmp/nostr-relay.log
        exit 1
    fi
    sleep 0.5
done
echo "[entrypoint] Nostr relay is up"

# Tests share a single nostr relay and each spawns its own bitcoind; running
# them in parallel risks offerbook cross-talk and resource exhaustion.
exec cargo test --test e2e --features integration-test -- --nocapture --test-threads=1

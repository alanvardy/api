#!/usr/bin/env bash
echo "=== UPDATE MIGRATIONS ===" &&
cargo sqlx prepare &&
echo "=== CHECK ===" &&
cargo check &&
echo "=== CLIPPY ===" &&
cargo clippy -- -D warnings &&
echo "=== TEST ===" &&
cargo nextest run &&
echo "=== FORGOTTEN TODOS ===" &&
# Requires ripgrep
if rg -i -s -g '*.rs' 'TODO:|todo:|FIXME|fixme|dbg!|DEBUG:|FIXTURE:|TODO\s|todo\s' .; then
    exit 1
fi
echo "=== SUCCESS ===" &&
echo "=== Done ===."

#!/bin/bash
set -e # Exit immediately if a command fails

# The daemon will handle logging so we make these silent to avoid duplicate
# messages.
export LUCKY_LOG_LEVEL=off
./bin/lucky daemon start --ignore-already-running
./bin/lucky daemon trigger-hook {hook_name}
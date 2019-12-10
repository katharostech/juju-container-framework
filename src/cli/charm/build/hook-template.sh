#!/bin/bash
set -e # Exit immediately if a command fails

if [ ! -f ./bin/lucky ]; then
    # TODO: Download or install Lucky
    echo "TODO: Download or install Lucky"
fi

# Start the Lucky daemon
./bin/lucky daemon start --ignore-already-running

# Trigger the `{hook_name}` hook
./bin/lucky daemon trigger-hook {hook_name}
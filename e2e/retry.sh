#!/usr/bin/env bash

function fail {
  echo $1 >&2
  exit 1
}

n=1
max=5
while true; do
  "$@" && break || {
      if [[ $n -lt $max ]]; then
        ((n++))
        echo "Command failed. Attempt $n/$max:"
        sleep 2;
      else
        fail "The command has failed after $n attempts."
      fi
}
done

#!/usr/bin/env bash

function fail {
  echo $1 >&2
  exit 1
}

command="kubectl get secret test-rsecret -o jsonpath={.data.$1}"

n=1
max=10
while true; do
  $command | grep $2 && break || {
      if [[ $n -lt $max ]]; then
        ((n++))
        echo "Command failed. Attempt $n/$max:"
        sleep 2;
      else
        fail "The command has failed after $n attempts."
      fi
}
done

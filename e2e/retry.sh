#!/usr/bin/env bash

function fail {
  echo $1 >&2
  exit 1
}

command="kubectl get secret test-rsecret -o jsonpath={.data.$1}"

n=1
max=20
while true; do
  kubectl get secret test-rsecret -o jsonpath={.data.$1} | grep $2 && break || {
      if [[ $n -lt $max ]]; then
        ((n++))
        echo "retry failed for path: <$1> check for : <$2>. Attempt $n/$max:"
        sleep 2;
      else
        fail "The command has failed after $n attempts."
      fi
}
done

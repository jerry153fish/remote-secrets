#!/usr/bin/env bash

set -x

e2e/retry.sh test-rsecret-plaintext dGVzdC1yc2VjcmV0LXBsYWludGV4dC12YWx1ZQ==
e2e/retry.sh test-rsecret-ssm-param VGVzdDE=
e2e/retry.sh test-rsecret-secretmanager dGVzdEF3c1NlY3JldE1hbmFnZXI=
# nested json string 
e2e/retry.sh ssmName dGVzdA==
e2e/retry.sh objectName b2JqZWN0TmFtZQ==
#!/usr/bin/env bash

set -xe

# plain text
e2e/retry.sh test-rsecret-plaintext dGVzdC1yc2VjcmV0LXBsYWludGV4dC12YWx1ZQ==

# ssm string value
e2e/retry.sh test-rsecret-ssm-param VmljaQ==

# ssm nested json value 
e2e/retry.sh ssmName dGVzdA==
e2e/retry.sh objectName b2JqZWN0TmFtZQ==

# secret manager string value
e2e/retry.sh test-rsecret-secretmanager VmljZA==

# secret manager nested json value
e2e/retry.sh ssmName dGVzdA==
e2e/retry.sh srmTest b2JqZWN0TmFtZQ==

# cloudformation nested outputs
e2e/retry.sh S3Bucket UzNCdWNrZXQ=
e2e/retry.sh S3TestPoint VGVzdFBvaW50

# cloudformation specific path
e2e/retry.sh test-cfn-stack UzNCdWNrZXQ=

# vault string data
e2e/retry.sh vault-string InZhdWx0U3RyaW5nIg==

# vault nested json
e2e/retry.sh vaultJson1 dmF1bHRKc29uMQ==
e2e/retry.sh vaultJson2 dmF1bHRKc29uMg==
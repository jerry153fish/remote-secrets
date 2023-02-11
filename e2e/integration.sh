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
e2e/retry.sh S3BucketSecureURL aHR0cHM6Ly9teXRlc3RzdGFjay1zM2J1Y2tldC0yNmZiYzA3ZS5zMy5sb2NhbGhvc3QubG9jYWxzdGFjay5jbG91ZA==

# specific path
e2e/retry.sh test-cfn-stack UzNCdWNrZXQ=
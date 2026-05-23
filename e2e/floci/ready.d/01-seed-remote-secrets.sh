#!/bin/sh
set -eu

aws ssm put-parameter \
  --name MyStringParameter \
  --type String \
  --value Vici \
  --overwrite

aws ssm put-parameter \
  --name MyJsonParameter \
  --type String \
  --value '{ "ssmName": "test", "objectName": "objectName"}' \
  --overwrite

aws secretsmanager create-secret \
  --name MyTestSecret \
  --secret-string Vicd || \
aws secretsmanager put-secret-value \
  --secret-id MyTestSecret \
  --secret-string Vicd

aws secretsmanager create-secret \
  --name MyJsonSecret \
  --secret-string '{ "srmName": "test", "srmTest": "objectName"}' || \
aws secretsmanager put-secret-value \
  --secret-id MyJsonSecret \
  --secret-string '{ "srmName": "test", "srmTest": "objectName"}'

aws cloudformation create-stack \
  --stack-name MyTestStack \
  --template-body file:///etc/floci/templates/mock-cfn.yaml || \
aws cloudformation update-stack \
  --stack-name MyTestStack \
  --template-body file:///etc/floci/templates/mock-cfn.yaml || true

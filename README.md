## remote-secrets
[![Main CI](https://github.com/jerry153fish/remote-secrets/actions/workflows/ci.yaml/badge.svg)](https://github.com/jerry153fish/remote-secrets/actions/workflows/ci.yaml)

Remote Secrets allows you to use remote secret management systems, SSM parameter store, application configuration, Cloudformation stack outputs, and more to add a single secret in k8s.


## How to use it

### Install CRD

1.  from url

```
kubectl apply -f https://raw.githubusercontent.com/jerry153fish/remote-secrets/main/config/crd.yaml
```

2. from source 

```
make crd
```

### Install controller

1. from url

```
kubectl apply -f https://raw.githubusercontent.com/jerry153fish/remote-secrets/main/config/default/manager.yaml
```

2. from source

```
make manifest
```

### Create simple secret

1. simple plain text secrets

```
kubectl apply -f https://raw.githubusercontent.com/jerry153fish/remote-secrets/main/config/simple/plaintext.yaml
```

It will just add a s simple plain text as the backend

```
apiVersion: jerry153fish.com/v1beta1
kind: RSecret # Identifier of the resource type.
metadata:
  name: test-plaintext # Name of the "RSecret" custom resource instance, may be changed to your liking
  namespace: default # Namespace must exist and account in KUBECONFIG must have sufficient permissions
spec:
  description: "test plaintext secrets" # Optional description of the resource
  resources:
    - backend: Plaintext
      data:
        - value: test-rsecret-plaintext-value
          key: test-rsecret-plaintext
        - value: test-rsecret-plaintext-value-1
          key: test-rsecret-plaintext-1

```


### Configure backends access

if you want to add other backend you need to configure the backend access with k8s secret eg:

```

apiVersion: v1
kind: Secret
metadata:
  name: remote-secrets
  namespace: remote-secrets
type: Opaque
stringData:
  APP: "remote-secrets"
#  AWS_ACCESS_KEY_ID: "aaa"
#  AWS_SECRET_ACCESS_KEY: "secret"
#  AWS_REGION: "ap-southeast-2"
#  LOCALSTACK_URL: "http://dockerhost:4572"
#  TEST_ENV: "true"
```

1. aws backend

The example above uses the [environment variable](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-envvars.html) to configure the AWS credentials. However, the remote-secrets is using [aws-config](https://crates.io/crates/aws-config) which utilizes the AWS credential provider chain, 
so you can configure the credentials the way you want.

### AWS Parameter Store

> ensure you have correct access to SSM

1. add the parameter 

```
aws ssm put-parameter --name MyStringParameter --type "String" --value "Vici"
```

2. add a SSM backend

```
    - backend: SSM
      data:
        - value: MyStringParameter
          key: test-rsecret-ssm-param
```

### AWS Secret Manager

> ensure you have correct access to Secret manager

1. add the secret

```
aws secretsmanager create-secret --name MyTestSecret --secret-string "Vicd" 
```

2. add a secret manager backend

```
    - backend: SecretManager
      data:
        - value: MyTestSecret
          key: test-rsecret-secretmanager
```

### AWS Cloudformation outputs

> ensure you have correct access to Cloudformation

1. add a simple cloudformation stack

```
aws cloudformation create-stack --stack-name MyTestStack --template-body file://e2e/mock-cfn.yaml
```

2. add as a cloudformation backend

```
    - backend: Cloudformation
      data:
        - value: MyTestStack # import all the outputs
        - value: MyTestStack # import specific output key
          key: test-cfn-stack
          remote_path: S3Bucket
```

## Development

### Requirements

1. [kind](https://kind.sigs.k8s.io/) or any other local k8s cluster
2. [docker](https://www.docker.com/) 

### Useful commands

1. help

```
make help
```

2. bring up local mock environment

```
make mock-env
```

3. create a local k8s cluster using kind

```
make kind-cluster
```

3. initialize test data

```
make init-test
```

4. run test

```
make test
```

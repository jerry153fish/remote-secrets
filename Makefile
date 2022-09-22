# VERSION defines the project version for the bundle.
# Update this value when you upgrade the version of your project.
# To re-generate a bundle for another specific version without changing the standard setup, you can:
# - use the VERSION as arg of the bundle target (e.g make bundle VERSION=0.0.2)
# - use environment variables to overwrite this value (e.g export VERSION=0.0.2)
VERSION ?= 0.0.1

# IMAGE_TAG_BASE defines the docker.io namespace and part of the image name for remote images.
# This variable is used to construct full image tags for bundle and catalog images.
#
# For example, running 'make bundle-build bundle-push catalog-build catalog-push' will build and push both
# jerry153fish.com/aws-secrets-bundle:$VERSION and jerry153fish.com/aws-secrets-catalog:$VERSION.
IMAGE_TAG_BASE ?= jerry153fish.com/aws-secrets

# BUNDLE_IMG defines the image:tag used for the bundle.
# You can use it as an arg. (E.g make bundle-build BUNDLE_IMG=<some-registry>/<project-name-bundle>:<tag>)
BUNDLE_IMG ?= $(IMAGE_TAG_BASE)-bundle:v$(VERSION)

# Image URL to use all building/pushing image targets
IMG ?= controller:local


# Setting SHELL to bash allows bash commands to be executed by recipes.
# This is a requirement for 'setup-envtest.sh' in the test target.
# Options are set to exit when a recipe line exits non-zero or a piped command fails.
SHELL = /usr/bin/env bash -o pipefail
.SHELLFLAGS = -ec

all: build

##@ General

# The help target prints out all targets with their descriptions organized
# beneath their categories. The categories are represented by '##@' and the
# target descriptions by '##'. The awk commands is responsible for reading the
# entire set of makefiles included in this invocation, looking for lines of the
# file as xyz: ## something, and then pretty-format the target and help. Then,
# if there's a line with ##@ something, that gets pretty-printed as a category.
# More info on the usage of ANSI control characters for terminal formatting:
# https://en.wikipedia.org/wiki/ANSI_escape_code#SGR_parameters
# More info on the awk command:
# http://linuxcommand.org/lc3_adv_awk.php

help: ## Display this help.
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z_0-9-]+:.*?##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

##@ Development

manifest: ## Generate the manifest file.
	kubectl kustomize config/default | kubectl apply -f -

manifest-clean: ## Clean the manifest file.
	kubectl kustomize config/default | kubectl delete -f -

manifest-local: ## Generate the manifest file for local testing.
	kubectl kustomize config/local | kubectl apply -f -

manifest-local-clean: ## Clean the manifest file for local testing.
	kubectl kustomize config/local | kubectl delete -f -

crdgen: ## Generate CRDs
	cargo run --bin crd > config/crd.yaml

test: ## run the tests
	export TEST_ENV=true && cargo test --all-targets -- --nocapture

init-test: ## init the test environment
	mkdir debug || true
	aws ssm put-parameter --endpoint-url http://localhost:4566 --name MyStringParameter --type "String" --value "Vici" --overwrite > /dev/null || true
	aws secretsmanager create-secret --endpoint-url http://localhost:4566 --name MyTestSecret --secret-string "Vicd" > /dev/null || true
	aws cloudformation create-stack --endpoint-url http://localhost:4566 --stack-name MyTestStack --template-body file://e2e/mock-cfn.yaml > debug/create-stack-result.json || true
	curl -H "X-Vault-Token: vault-plaintext-root-token" -H "Content-Type: application/json" -X POST -d '{"data":{"value":"bar"}}' http://127.0.0.1:8200/v1/secret/data/baz || true
	curl -H "X-Vault-Token: vault-plaintext-root-token" -H "Content-Type: application/json" -X POST -d '{"data":{"value":{"test": "aaa"}}}' http://127.0.0.1:8200/v1/secret/data/foo || true

mock-env: ## intialize the test environment locally
	docker compose -f e2e/services.yaml up -d

kind-cluster: ## create a kind cluster
	kind create cluster --name local || true

fmt: ## format the code
	cargo fmt --all

doc: ## generate the documentation
	cargo doc

##@ Build

controller: ## Run a controller from your host.
	cargo run --bin controller

docker-build: ## Build docker image with the manager.
	docker build -t ${IMG} .

##@ Deployment

install-crd: ## install the CRDs
	kubectl apply -f config/crd.yaml

kind-image-load: ## load the kind image
	kind load docker-image ${IMG} --name local

install-local: install-crd manifest-local ## install the local cluster

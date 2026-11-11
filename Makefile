# =============================================================================
# Project Configuration
# =============================================================================

# Project metadata
PROJECT_NAME := wol-relay
DESCRIPTION := Wake-on-LAN relay service
DOCKER_IMAGE ?= ghcr.io/wty2019wty/$(PROJECT_NAME)

# Directories
ROOT_DIR := $(shell dirname $(realpath $(lastword $(MAKEFILE_LIST))))
DIST_DIR := $(ROOT_DIR)/dist

# Colors
BLUE := \033[34m
GREEN := \033[32m
YELLOW := \033[33m
RED := \033[31m
MAGENTA := \033[35m
CYAN := \033[36m
WHITE := \033[37m
BOLD := \033[1m
RESET := \033[0m

# =============================================================================
# @Linters
# =============================================================================

.PHONY: lint
lint: lint-fmt lint-clippy ## Run all linters

.PHONY: lint-fix
lint-fix: lint-fix-fmt ## Try to fix lint issues

.PHONY: lint-fmt
lint-fmt: ## Check formatting
	cargo fmt --check

.PHONY: lint-fix-fmt
lint-fix-fmt: ## Fix formatting
	cargo fmt

.PHONY: lint-clippy
lint-clippy: ## Run clippy linter
	cargo clippy --all-targets -- -D warnings

# =============================================================================
# @Dependencies
# =============================================================================

.PHONY: deps-update
deps-update: ## Update dependencies
	cargo update

# =============================================================================
# Directory Creation
# =============================================================================

$(DIST_DIR):
	mkdir -p $@

# =============================================================================
# @Build
# =============================================================================

.PHONY: run
run: ## Run the main program
	cargo run --

.PHONY: build
build: $(DIST_DIR) ## Build the binary
	cargo build --release
	cp target/release/wol-relay $(DIST_DIR)/wol-relay

.PHONY: build-docker
build-docker: ## Build the Docker image
	docker build -t $(DOCKER_IMAGE) .

# =============================================================================
# @Tests
# =============================================================================

.PHONY: test
test: ## Run tests
	cargo test

# =============================================================================
# @Help
# =============================================================================

.PHONY: help
help: ## Display this help message
	@echo "$(CYAN)$(BOLD)$(PROJECT_NAME) - $(DESCRIPTION)$(RESET)"
	@awk 'BEGIN { FS = ":.*?##" } \
		/^[a-zA-Z0-9._-]+:.*?##/ { printf "  $(CYAN)%-15s$(RESET) %s\n", $$1, $$2 } \
		/^# @/ { printf "\n$(MAGENTA)%s$(RESET)\n\n", substr($$0, 4) }' \
		$(MAKEFILE_LIST)
	@echo

.DEFAULT_GOAL := help

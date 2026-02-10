NAME	:= pog

# Podman commands
POD_RUN	:= podman run --rm -v $(PWD):/usr/local/src $(NAME)-builder build --release

# Logger
define log_info
	echo -e "[\033[0;33m*\033[0m] $(1)"
endef

define log_success
	echo -e "[\033[0;32m+\033[0m] Done"
endef

all: release

debug: POD_RUN += --features debug
debug: build

release: build
	@ $(call log_info,Stripping binary...)
	@ strip target/release/$(NAME)
	@ $(call log_success)

build: clean
	@ $(call log_info,Compiling...)
	@ $(POD_RUN)
	@ $(call log_success)

test:
	@ $(call log_info,Running tests...)
	@ podman run --rm -v $(PWD):/usr/local/src $(NAME)-builder test --workspace
	@ $(call log_success)

pod-build:
	@ $(call log_info,Building Podman image...)
	@ podman build --quiet -t $(NAME)-builder . --format docker
	@ $(call log_success)

pod-clean:
	@ $(call log_info,Deleting Podman image...)
	@ podman image rm $(NAME)-builder
	@ $(call log_success)

clean:
	@ $(call log_info,Cleaning build artifacts)
	@ rm -rf target
	@ $(call log_success)

.PHONY: all release debug build test pod-build pod-clean clean

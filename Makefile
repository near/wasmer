SHELL=/usr/bin/env bash


#####
#
# The Matrix
#
#####

# The matrix is the product of the following columns:
#
# |------------|-----------|----------|--------------|-------|
# | Compiler   ⨯ Engine    ⨯ Platform ⨯ Architecture ⨯ libc  |
# |------------|-----------|----------|--------------|-------|
# | Cranelift  | Universal | Linux    | amd64        | glibc |
# | LLVM       | Dylib     | Darwin   | aarch64      | musl  |
# | Singlepass | Staticlib | Windows  |              |       |
# |------------|-----------|----------|--------------|-------|
#
# Here is what works and what doesn't:
#
# * Cranelift with the Universal engine works everywhere,
#
# * Cranelift with the Dylib engine works on Linux+Darwin/`amd64`, but
#   it doesn't work on */`aarch64` or Windows/*.
#
# * LLVM with the Universal engine works on Linux+Darwin/`amd64`,
#   but it doesn't work on */`aarch64` or Windows/*.
#
# * LLVM with the Dylib engine works on
#   Linux+Darwin/`amd64`+`aarch64`, but it doesn't work on Windows/*.
#
# * Singlepass with the Universal engine works on Linux+Darwin/`amd64`, but
#   it doesn't work on */`aarch64` or Windows/*.
#
# * Singlepass with the Dylib engine doesn't work because it doesn't
#   know how to output object files for the moment.
#
# * Windows isn't tested on `aarch64`, that's why we consider it's not
#   working, but it might possibly be.


#####
#
# Define the “Platform” and “Architecture” columns of the matrix.
#
#####


IS_DARWIN := 0
IS_LINUX := 0
IS_WINDOWS := 0
IS_AMD64 := 0
IS_AARCH64 := 0

# Test Windows apart because it doesn't support `uname -s`.
ifeq ($(OS), Windows_NT)
	# We can assume it will likely be in amd64.
	IS_AMD64 := 1
	IS_WINDOWS := 1
else
	# Platform
	uname := $(shell uname -s)

	ifeq ($(uname), Darwin)
		IS_DARWIN := 1
	else ifeq ($(uname), Linux)
		IS_LINUX := 1
	else
		# We use spaces instead of tabs to indent `$(error)`
		# otherwise it's considered as a command outside a
		# target and it will fail.
                $(error Unrecognized platform, expect `Darwin`, `Linux` or `Windows_NT`)
	endif

	# Architecture
	uname := $(shell uname -m)

	ifeq ($(uname), x86_64)
		IS_AMD64 := 1
	else ifneq (, $(filter $(uname), aarch64 arm64))
		IS_AARCH64 := 1
	else
		# We use spaces instead of tabs to indent `$(error)`
		# otherwise it's considered as a command outside a
		# target and it will fail.
                $(error Unrecognized architecture, expect `x86_64`, `aarch64` or `arm64`)
	endif

	# Libc
	LIBC ?= $(shell ldd 2>&1 | grep -o musl | head -n1)
endif


#####
#
# Define the “Compiler” column of the matrix.
#
#####


# Variables that can be overriden by the users to force to enable or
# to disable a specific compiler.
ENABLE_CRANELIFT ?=
ENABLE_LLVM ?=
ENABLE_SINGLEPASS ?=

# Which compilers we build. These have dependencies that may not be on the system.
compilers :=

##
# Cranelift
##

# If the user didn't disable the Cranelift compiler…
ifneq ($(ENABLE_CRANELIFT), 0)
	# … then it can always be enabled.
	compilers += cranelift
	ENABLE_CRANELIFT := 1
endif

##
# LLVM
##

# If the user didn't disable the LLVM compiler…
ifneq ($(ENABLE_LLVM), 0)
	# … then maybe the user forced to enable the LLVM compiler.
	ifeq ($(ENABLE_LLVM), 1)
		LLVM_VERSION := $(shell llvm-config --version)
		compilers += llvm
	# … otherwise, we try to autodetect LLVM from `llvm-config`
	else ifneq (, $(shell which llvm-config 2>/dev/null))
		LLVM_VERSION := $(shell llvm-config --version)

		# If findstring is not empty, then it have found the value
		ifneq (, $(findstring 13,$(LLVM_VERSION)))
			compilers += llvm
		else ifneq (, $(findstring 12,$(LLVM_VERSION)))
			compilers += llvm
		endif
	# … or try to autodetect LLVM from `llvm-config-<version>`.
	else
		ifneq (, $(shell which llvm-config-13 2>/dev/null))
			LLVM_VERSION := $(shell llvm-config-13 --version)
			compilers += llvm
		else ifneq (, $(shell which llvm-config-12 2>/dev/null))
			LLVM_VERSION := $(shell llvm-config-12 --version)
			compilers += llvm
		endif
	endif
endif

exclude_tests := --exclude wasmer-cli
# We run integration tests separately (it requires building the c-api)
exclude_tests += --exclude wasmer-integration-tests-cli
exclude_tests += --exclude wasmer-integration-tests-ios

ifneq (, $(findstring llvm,$(compilers)))
	ENABLE_LLVM := 1
else
	# We exclude LLVM from our package testing
	exclude_tests += --exclude wasmer-compiler-llvm
endif

##
# Singlepass
##

# If the user didn't disable the Singlepass compiler…
ifneq ($(ENABLE_SINGLEPASS), 0)
	# … then maybe the user forced to enable the Singlepass compiler.
	ifeq ($(ENABLE_SINGLEPASS), 1)
		compilers += singlepass
	# … otherwise, we try to check whether Singlepass works on this host.
	else ifneq (, $(filter 1, $(IS_DARWIN) $(IS_LINUX) $(IS_WINDOWS)))
		ifeq ($(IS_AMD64), 1)
			compilers += singlepass
		endif
	endif
endif

ifneq (, $(findstring singlepass,$(compilers)))
	ENABLE_SINGLEPASS := 1
endif

##
# Clean the `compilers` variable.
##

compilers := $(strip $(compilers))


#####
#
# Define the “Engine” column of the matrix.
#
#####


# The engine is part of a pair of kind (compiler, engine). All the
# pairs are stored in the `compilers_engines` variable.
compilers_engines :=

##
# The Cranelift case.
##

ifeq ($(ENABLE_CRANELIFT), 1)
	compilers_engines += cranelift-universal

	ifneq (, $(filter 1, $(IS_DARWIN) $(IS_LINUX)))
		ifeq ($(IS_AMD64), 1)
			ifneq ($(LIBC), musl)
				compilers_engines += cranelift-dylib
			endif
		else ifeq ($(IS_AARCH64), 1)
			# The object crate doesn't support yet Darwin + Aarch64 relocations
			ifneq ($(IS_DARWIN), 1)
				compilers_engines += cranelift-dylib
			endif
		endif
	endif
endif

##
# The LLVM case.
##

ifeq ($(ENABLE_LLVM), 1)
	ifneq (, $(filter 1, $(IS_DARWIN) $(IS_LINUX)))
		ifeq ($(IS_AMD64), 1)
			compilers_engines += llvm-universal
			compilers_engines += llvm-dylib
		else ifeq ($(IS_AARCH64), 1)
			compilers_engines += llvm-universal
			compilers_engines += llvm-dylib
		endif
	endif
endif

##
# The Singlepass case.
##

ifeq ($(ENABLE_SINGLEPASS), 1)
	ifneq (, $(filter 1, $(IS_DARWIN) $(IS_LINUX)))
		ifeq ($(IS_AMD64), 1)
			compilers_engines += singlepass-universal
		endif
	endif
endif

# Clean the `compilers_engines` variable.
compilers_engines := $(strip $(compilers_engines))


#####
#
# Cargo features.
#
#####

# Small trick to define a space and a comma.
space := $() $()
comma := ,

# Define the compiler Cargo features for all crates.
compiler_features := --features $(subst $(space),$(comma),$(compilers))

#####
#
# Display information.
#
#####

ifneq (, $(filter 1, $(IS_DARWIN) $(IS_LINUX)))
	bold := $(shell tput bold 2>/dev/null || echo -n '')
	green := $(shell tput setaf 2 2>/dev/null || echo -n '')
	yellow := $(shell tput setaf 3 2>/dev/null || echo -n '')
	reset := $(shell tput sgr0 2>/dev/null || echo -n '')
endif

HOST_TARGET=$(shell rustup show | grep 'Default host: ' | cut -d':' -f2 | tr -d ' ')

TARGET_DIR := target/release

ifneq (, $(TARGET))
	TARGET_DIR := target/$(TARGET)/release
endif

$(info -----------)
$(info $(bold)$(green)INFORMATION$(reset))
$(info -----------)
$(info )
$(info Host Target: `$(bold)$(green)$(HOST_TARGET)$(reset)`.)
ifneq (, $(TARGET))
	# We use spaces instead of tabs to indent `$(info)`
	# otherwise it's considered as a command outside a
	# target and it will fail.
        $(info Build Target: $(bold)$(green)$(TARGET)$(reset) $(yellow)($(TARGET_DIR))$(reset))
endif
ifneq (, $(LIBC))
	# We use spaces instead of tabs to indent `$(info)`
	# otherwise it's considered as a command outside a
	# target and it will fail.
        $(info C standard library: $(bold)$(green)$(LIBC)$(reset))
endif
$(info Enabled Compilers: $(bold)$(green)$(subst $(space),$(reset)$(comma)$(space)$(bold)$(green),$(compilers))$(reset).)
$(info Testing the following compilers & engines:)
$(info   * API: $(bold)$(green)${compilers_engines}$(reset),)
$(info Cargo features:)
$(info   * Compilers: `$(bold)$(green)${compiler_features}$(reset)`.)
$(info Rust version: $(bold)$(green)$(shell rustc --version)$(reset).)
$(info NodeJS version: $(bold)$(green)$(shell node --version)$(reset).)
ifeq ($(ENABLE_LLVM), 1)
        $(info LLVM version: $(bold)$(green)${LLVM_VERSION}$(reset).)
endif
$(info )
$(info )
$(info --------------)
$(info $(bold)$(green)RULE EXECUTION$(reset))
$(info --------------)
$(info )
$(info )

#####
#
# Configure `sed -i` for a cross-platform usage.
#
#####

SEDI ?=

ifeq ($(IS_DARWIN), 1)
	SEDI := "-i ''"
else ifeq ($(IS_LINUX), 1)
	SEDI := "-i"
endif

#####
#
# Building.
#
#####

bench:
	cargo bench $(compiler_features)

# For best results ensure the release profile looks like the following
# in Cargo.toml:
# [profile.release]
# opt-level = 'z'
# debug = false
# debug-assertions = false
# overflow-checks = false
# lto = true
# panic = 'abort'
# incremental = false
# codegen-units = 1
# rpath = false
ifeq ($(IS_DARWIN), 1)
	strip -u target/$(HOST_TARGET)/release/wasmer-headless
else ifeq ($(IS_WINDOWS), 1)
	strip --strip-unneeded target/$(HOST_TARGET)/release/wasmer-headless.exe
else
	strip --strip-unneeded target/$(HOST_TARGET)/release/wasmer-headless
endif

#####
#
# Testing.
#
#####

test: test-compilers test-packages test-examples

test-compilers:
	cargo test --release --tests $(compiler_features)

test-packages:
	cargo test --all --release $(exclude_tests)
	cargo test --manifest-path lib/compiler-cranelift/Cargo.toml --release --no-default-features --features=std
	cargo test --manifest-path lib/compiler-singlepass/Cargo.toml --release --no-default-features --features=std

#####
#
# Testing compilers.
#
#####

test-compilers-compat: $(foreach compiler,$(compilers),test-$(compiler))

test-singlepass-dylib:
	cargo test --release --tests $(compiler_features) -- singlepass::dylib

test-singlepass-universal:
	cargo test --release --tests $(compiler_features) -- singlepass::universal

test-cranelift-dylib:
	cargo test --release --tests $(compiler_features) -- cranelift::dylib

test-cranelift-universal:
	cargo test --release --tests $(compiler_features) -- cranelift::universal

test-llvm-dylib:
	cargo test --release --tests $(compiler_features) -- llvm::dylib

test-llvm-universal:
	cargo test --release --tests $(compiler_features) -- llvm::universal

test-singlepass: $(foreach singlepass_engine,$(filter singlepass-%,$(compilers_engines)),test-$(singlepass_engine))

test-cranelift: $(foreach cranelift_engine,$(filter cranelift-%,$(compilers_engines)),test-$(cranelift_engine))

test-llvm: $(foreach llvm_engine,$(filter llvm-%,$(compilers_engines)),test-$(llvm_engine))

test-examples:
	cargo test --release $(compiler_features) --examples

test-integration:
	cargo test -p wasmer-integration-tests-cli

test-integration-ios:
	cargo test -p wasmer-integration-tests-ios

#####
#
# Packaging.
#
#####

package-docs: build-docs
	mkdir -p "package/docs/crates"
	cp -R target/doc/ package/docs/crates
	echo '<meta http-equiv="refresh" content="0; url=crates/wasmer/index.html">' > package/docs/index.html
	echo '<meta http-equiv="refresh" content="0; url=wasmer/index.html">' > package/docs/crates/index.html

distribution: package
	cp LICENSE package/LICENSE
	cp ATTRIBUTIONS.md package/ATTRIBUTIONS
	mkdir -p dist
ifeq ($(IS_WINDOWS), 1)
	iscc scripts/windows-installer/wasmer.iss
	cp scripts/windows-installer/WasmerInstaller.exe dist/
endif
	tar -C package -zcvf wasmer.tar.gz bin lib include LICENSE ATTRIBUTIONS
	mv wasmer.tar.gz dist/

#####
#
# Installating (for Distros).
#
#####

DESTDIR ?= /usr/local

install: install-wasmer install-pkgconfig install-misc

install-wasmer:
	install -Dm755 target/release/wasmer $(DESTDIR)/bin/wasmer

install-misc:
	install -Dm644 LICENSE "$(DESTDIR)"/share/licenses/wasmer/LICENSE

install-pkgconfig:
	# Make sure WASMER_INSTALL_PREFIX is set during build
	unset WASMER_DIR; \
	if pc="$$(target/release/wasmer config --pkg-config 1>/dev/null 2>/dev/null)"; then \
		echo "$$pc" | install -Dm644 /dev/stdin "$(DESTDIR)"/lib/pkgconfig/wasmer.pc; \
	else \
		echo 1>&2 "WASMER_INSTALL_PREFIX was not set during build, not installing wasmer.pc"; \
	fi

install-wasmer-headless-minimal:
	install -Dm755 target/release/wasmer-headless $(DESTDIR)/bin/wasmer-headless

#####
#
# Miscellaneous.
#
#####

# Updates the spectests from the repo
update-testsuite:
	git subtree pull --prefix tests/wast/spec https://github.com/WebAssembly/testsuite.git master --squash

lint-packages: RUSTFLAGS += -D dead-code -D nonstandard-style -D unused-imports -D unused-mut -D unused-variables -D unused-unsafe -D unreachable-patterns -D bad-style -D improper-ctypes -D unused-allocation -D unused-comparisons -D while-true -D unconditional-recursion -D bare-trait-objects # TODO: add `-D missing-docs` # TODO: add `-D function_item_references` (not available on Rust 1.47, try when upgrading)
lint-packages:
	RUSTFLAGS="${RUSTFLAGS}" cargo clippy --all $(exclude_tests)
	RUSTFLAGS="${RUSTFLAGS}" cargo clippy --manifest-path fuzz/Cargo.toml $(compiler_features)

lint-formatting:
	cargo fmt --all -- --check
	cargo fmt --manifest-path fuzz/Cargo.toml -- --check

lint: lint-formatting lint-packages

install-local: package
	tar -C ~/.wasmer -zxvf wasmer.tar.gz

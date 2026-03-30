PROJECT := memcontainers
KERNEL := crates/kernel
HOST := crates/host-wasmtime
WASM_TARGET := wasm32-unknown-unknown
KERNEL_WASM := target/$(WASM_TARGET)/release/kernel.wasm
HOST_BIN := target/release/host-wasmtime

BOLD := $(shell tput bold)
RESET := $(shell tput sgr0)

.DEFAULT_GOAL := help

.PHONY: +build-kernel # Build the kernel WASM module
+build-kernel:
	@echo "$(BOLD)Building kernel WASM...$(RESET)"
	@cargo build -p kernel --target $(WASM_TARGET) --release
	@echo "$(BOLD)Kernel WASM at $(KERNEL_WASM)$(RESET)"

.PHONY: +build-host # Build the host-wasmtime binary
+build-host:
	@echo "$(BOLD)Building host-wasmtime...$(RESET)"
	@cargo build -p host-wasmtime --release
	@echo "$(BOLD)Host binary at $(HOST_BIN)$(RESET)"

.PHONY: +build # Build both kernel and host
+build: +build-kernel +build-host
	@echo "$(BOLD)Build complete.$(RESET)"

.PHONY: +run # Run the host with the kernel
+run: +build
	@echo "$(BOLD)Running $(PROJECT)...$(RESET)"
	@$(HOST_BIN) --kernel $(KERNEL_WASM)

.PHONY: +dev # Build and run in development mode
+dev:
	@echo "$(BOLD)Building and running in dev mode...$(RESET)"
	@cargo build -p kernel --target $(WASM_TARGET)
	@cargo build -p host-wasmtime
	@./target/debug/host-wasmtime --kernel ./target/$(WASM_TARGET)/debug/kernel.wasm

.PHONY: +clean # Clean build artifacts
+clean:
	@echo "$(BOLD)Cleaning build artifacts...$(RESET)"
	@cargo clean

.PHONY: help # Display the help message
help:
	@echo "$(BOLD)Available targets:$(RESET)"
	@cat Makefile | grep '.PHONY: [a-z\+]' | sed 's/.PHONY: / /g' | sed 's/ #* / - /g'

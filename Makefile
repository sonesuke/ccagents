# Makefile for ccauto project

.PHONY: test coverage clean help

# Default target
help:
	@echo "Available targets:"
	@echo "  test         - Run tests"
	@echo "  coverage     - Generate LCOV coverage report"
	@echo "  clean        - Clean build artifacts and coverage reports"
	@echo "  help         - Show this help message"

# Run tests
test:
	cargo test

# Generate LCOV coverage report
coverage:
	cargo llvm-cov --lcov --output-path target/lcov.info

# Clean artifacts
clean:
	cargo clean
	rm -rf target/lcov.info target/coverage/
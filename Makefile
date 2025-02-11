.PHONY: test install

install:
	cargo install --path .
	# echo "\n🔧 Setting up FNPM shell integration..."
	# SHELL_RC="$${HOME}/.$(shell basename $$SHELL)rc"; \
	# echo "\n# FNPM Shell Integration" >> "$$SHELL_RC"; \
	# echo 'cd() { builtin cd "$$@" && if [ -d ".fnpm" ] && [ -f ".fnpm/aliases.sh" ]; then source .fnpm/aliases.sh; fi; }' >> "$$SHELL_RC"; \
	# echo "✅ FNPM shell integration installed in $$SHELL_RC"

test:
	cd ./test && \
	fnpm $(cmd)

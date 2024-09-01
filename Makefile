.PHONY: build

build:
	nix-shell shell.nix --run "cargo build"

run:
	nix-shell shell.nix --run "cargo shuttle run"

# https://github.com/shuttle-hq/shuttle/issues/703
clean-deploy:
	cargo shuttle project restart --idle-minutes 0
	cargo shuttle deploy --no-test --working-directory dep-installer-hack --ad
	cargo shuttle deploy --ad


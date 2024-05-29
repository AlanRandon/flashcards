.PHONY: build

build:
	nix-shell shell.nix --run "cargo build"

run:
	nix-shell shell.nix --run "cargo shuttle run"

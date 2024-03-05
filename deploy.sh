# https://github.com/shuttle-hq/shuttle/issues/703

cargo shuttle project restart
cargo shuttle deploy --no-test --working-directory dep-installer-hack --ad
cargo shuttle deploy --ad

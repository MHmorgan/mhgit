all : prepare

install :
	cargo install --path .

test : export CARGO_INCREMENTAL=0
test : export RUSTFLAGS=-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zno-landing-pads
test :
	rm ./target/debug/deps/*.gcda
	cargo test --all-features

coverage : test
	grcov ./target/debug/ -s . -t html --llvm --branch --ignore-not-existing -o ./target/debug/coverage/
	xdg-open ./target/debug/coverage/index.html

doc :
	cargo doc --offline --no-deps --open

# Prepare for release
prepare :
	cargo fix --workspace --edition-idioms --release
	cargo clippy --verbose

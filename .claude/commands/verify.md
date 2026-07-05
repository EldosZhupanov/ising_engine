Run:

cargo check

cargo test --release

cargo build --release --bins

cargo clippy --release -- -D warnings

If performance code changed:

RUSTFLAGS='-C target-cpu=native -C llvm-args=-pass-remarks=loop-vectorize' cargo build --release --lib

objdump -d target/release/libising_engine.rlib | grep -c ymm

Return PASS only if everything succeeds.

FUZZCHECK_RS="$PWD"
cargo install --path cargo-fuzzcheck --force && \
cd usage_tests && \
cargo new --lib basic_example && \
cd basic_example && \
cargo fuzzcheck init file://"$FUZZCHECK_RS" && \
cargo fuzzcheck run target1 fuzz

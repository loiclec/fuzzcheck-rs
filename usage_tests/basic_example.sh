cargo install --path cargo-fuzzcheck --force && \
cd usage_tests/basic_crate && \
cargo fuzzcheck tests::fuzz && \
test  $(cat fuzz/tests::fuzz/artifacts/*.json) = '[{"x":100,"y":{"C":{"x":false,"y":true}}},{"x":55,"y":{"C":{"x":true,"y":false}}},{"x":87,"y":{"C":{"x":false,"y":false}}}]'  && \
rm -r fuzz

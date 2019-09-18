use std::hash::BuildHasher;

/// A fast, non-cryptographically secure hash
#[derive(Clone, Copy)]
pub struct FuzzcheckHash {}

impl BuildHasher for FuzzcheckHash {
    type Hasher = seahash::SeaHasher;
    fn build_hasher(&self) -> Self::Hasher {
        // Ideally `ahash` would be used instead, but it takes too long to compile
        seahash::SeaHasher::default()
    }
}

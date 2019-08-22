use std::hash::BuildHasher;

#[derive(Clone, Copy)]
pub struct FuzzcheckHash {}

impl BuildHasher for FuzzcheckHash {
    type Hasher = seahash::SeaHasher;
    fn build_hasher(&self) -> Self::Hasher {
        seahash::SeaHasher::default()
    }
}

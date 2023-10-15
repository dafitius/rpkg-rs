pub struct Md5Engine;

impl Md5Engine {

    pub fn compute(data: &str) -> u64 {
        let digest = md5::compute(data);
        let mut hash = 0u64;
        for i in 1..8 {
            hash |= u64::from(digest[i]) << (8 * (7 - i));
        }
        hash
    }
}
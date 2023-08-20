use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;

pub trait RandomBytesProvider {
    fn get_bytes(&self, len: usize) -> Vec<u8>;
}

pub struct RandomBytesProviderImpl;

impl RandomBytesProvider for RandomBytesProviderImpl {
    fn get_bytes(&self, len: usize) -> Vec<u8> {
        let mut rng = ChaCha20Rng::from_entropy();
        let mut data: Vec<u8> = vec![0; len];
        rng.fill_bytes(&mut data);
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore]
    #[tokio::test]
    async fn random_base16_string_provider_test() {
        let p = RandomBytesProviderImpl {};
        let result = p.get_bytes(10);
        println!("{text}", text = hex::encode(result));
    }
}

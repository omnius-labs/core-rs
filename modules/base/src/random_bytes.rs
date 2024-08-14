use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;

pub trait RandomBytesProvider {
    fn get_bytes(&mut self, len: usize) -> Vec<u8>;
    fn fill_bytes(&mut self, bytes: &mut [u8]);
}

pub struct RandomBytesProviderImpl {
    rng: ChaCha20Rng,
}

impl RandomBytesProviderImpl {
    pub fn new() -> Self {
        let rng = ChaCha20Rng::from_entropy();
        Self { rng }
    }
}

impl Default for RandomBytesProviderImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl RandomBytesProvider for RandomBytesProviderImpl {
    fn get_bytes(&mut self, len: usize) -> Vec<u8> {
        let mut data: Vec<u8> = vec![0; len];
        self.rng.fill_bytes(&mut data);
        data
    }

    fn fill_bytes(&mut self, bytes: &mut [u8]) {
        self.rng.fill_bytes(bytes);
    }
}

pub struct FakeRandomBytesProvider {
    rng: ChaCha20Rng,
}

impl FakeRandomBytesProvider {
    pub fn new() -> Self {
        let rng = ChaCha20Rng::from_seed([0; 32]); // fixed seed
        Self { rng }
    }
}

impl Default for FakeRandomBytesProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl RandomBytesProvider for FakeRandomBytesProvider {
    fn get_bytes(&mut self, len: usize) -> Vec<u8> {
        let mut data: Vec<u8> = vec![0; len];
        self.rng.fill_bytes(&mut data);
        data
    }

    fn fill_bytes(&mut self, bytes: &mut [u8]) {
        self.rng.fill_bytes(bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore]
    #[tokio::test]
    async fn random_base16_string_provider_test() {
        let mut p = RandomBytesProviderImpl::new();
        let result = p.get_bytes(10);
        println!("{text}", text = hex::encode(result));
    }
}

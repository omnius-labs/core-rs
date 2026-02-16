use rand::{
    SeedableRng,
    rngs::{ChaCha20Rng, SysRng},
};
use rand_core::{Rng, UnwrapErr};

pub trait RandomBytesProvider {
    fn get_bytes(&mut self, len: usize) -> Vec<u8>;
    fn fill_bytes(&mut self, bytes: &mut [u8]);
}

pub struct RandomBytesProviderChaCha20 {
    rng: ChaCha20Rng,
}

impl RandomBytesProviderChaCha20 {
    pub fn new() -> Self {
        let mut sys_rng = UnwrapErr(SysRng);
        let rng = ChaCha20Rng::from_rng(&mut sys_rng);
        Self { rng }
    }
}

impl Default for RandomBytesProviderChaCha20 {
    fn default() -> Self {
        Self::new()
    }
}

impl RandomBytesProvider for RandomBytesProviderChaCha20 {
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
        let mut p = RandomBytesProviderChaCha20::new();
        let result = p.get_bytes(10);
        println!("{text}", text = hex::encode(result));
    }
}

use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;

pub enum RandomBase16StringCase {
    Lower,
    Upper,
}

pub trait RandomStringGenerator {
    fn gen(&self) -> String;
}

pub struct RandomBase16StringProvider {
    pub len: usize,
    pub string_case: RandomBase16StringCase,
}

impl RandomBase16StringProvider {
    #[allow(unused)]
    fn new(len: usize, string_case: RandomBase16StringCase) -> Self {
        Self { len, string_case }
    }
}

impl RandomStringGenerator for RandomBase16StringProvider {
    fn gen(&self) -> String {
        let mut rng = ChaCha20Rng::from_entropy();
        let mut data: Vec<u8> = vec![0; self.len];
        rng.fill_bytes(&mut data);

        match self.string_case {
            RandomBase16StringCase::Lower => hex::encode(data),
            RandomBase16StringCase::Upper => hex::encode_upper(data),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore]
    #[tokio::test]
    async fn random_base16_string_provider_test() {
        let p = RandomBase16StringProvider::new(10, RandomBase16StringCase::Lower);
        let result = p.gen();
        println!("{result}");

        let p = RandomBase16StringProvider::new(10, RandomBase16StringCase::Upper);
        let result = p.gen();
        println!("{result}");
    }
}
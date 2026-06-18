pub trait Rng: Send + Sync {
    fn fill(&self, buf: &mut [u8]);
}

/// Production: delegates to the OS getrandom() syscall. Zero-sized, no lock.
pub struct SystemRng;

impl Rng for SystemRng {
    fn fill(&self, buf: &mut [u8]) {
        getrandom::getrandom(buf).expect("getrandom failed");
    }
}

/// Adapts a `&dyn Rng` into a `rand::RngCore` so rand distributions work directly with our trait.
pub struct RandAdapter<'a>(pub &'a dyn Rng);

impl rand::RngCore for RandAdapter<'_> {
    fn next_u32(&mut self) -> u32 {
        let mut buf = [0u8; 4];
        self.0.fill(&mut buf);
        u32::from_le_bytes(buf)
    }

    fn next_u64(&mut self) -> u64 {
        let mut buf = [0u8; 8];
        self.0.fill(&mut buf);
        u64::from_le_bytes(buf)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.0.fill(dest);
    }
}

impl rand::CryptoRng for RandAdapter<'_> {}

/// Tests: wraps `StdRng` behind our trait. Mutex only in tests; never used in production.
pub struct TestSeededRng(std::sync::Mutex<rand::rngs::StdRng>);

impl TestSeededRng {
    pub fn new(seed: u64) -> Self {
        use rand::SeedableRng;
        let mut seed_bytes = [0u8; 32];
        seed_bytes[..8].copy_from_slice(&seed.to_le_bytes());
        Self(std::sync::Mutex::new(rand::rngs::StdRng::from_seed(seed_bytes)))
    }

    pub fn from_seed(seed: [u8; 32]) -> Self {
        use rand::SeedableRng;
        Self(std::sync::Mutex::new(rand::rngs::StdRng::from_seed(seed)))
    }
}

impl Rng for TestSeededRng {
    fn fill(&self, buf: &mut [u8]) {
        use rand::RngCore;
        self.0.lock().unwrap().fill_bytes(buf);
    }
}

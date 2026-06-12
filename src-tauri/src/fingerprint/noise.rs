use rand::Rng;

/// Generate a random noise seed for AudioContext spoofing.
/// Returns an i32 in the range [-50, 50], matching Camoufox's canvas:aaOffset pattern.
pub fn audio_context_seed() -> i32 {
    let mut rng = rand::rng();
    rng.random_range(-50..=50)
}

/// Generate a random noise seed for ClientRects spoofing.
/// Returns an i32 in the range [-100, 100] for finer-grained offset control.
pub fn client_rects_seed() -> i32 {
    let mut rng = rand::rng();
    rng.random_range(-100..=100)
}

/// Generate a deterministic seed from a profile ID for consistent fingerprints
/// across launches of the same profile.
pub fn deterministic_seed(profile_id: &str, salt: &str) -> i32 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    format!("{}:{}", profile_id, salt).hash(&mut hasher);
    let hash = hasher.finish();
    (hash as i32).wrapping_rem(101)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_context_seed_range() {
        for _ in 0..100 {
            let seed = audio_context_seed();
            assert!((-50..=50).contains(&seed), "seed {seed} out of range");
        }
    }

    #[test]
    fn test_client_rects_seed_range() {
        for _ in 0..100 {
            let seed = client_rects_seed();
            assert!((-100..=100).contains(&seed), "seed {seed} out of range");
        }
    }

    #[test]
    fn test_deterministic_seed_consistency() {
        let s1 = deterministic_seed("test-uuid", "audio");
        let s2 = deterministic_seed("test-uuid", "audio");
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_deterministic_seed_different_salts() {
        let s1 = deterministic_seed("test-uuid", "audio");
        let s2 = deterministic_seed("test-uuid", "rects");
        assert_ne!(s1, s2);
    }
}

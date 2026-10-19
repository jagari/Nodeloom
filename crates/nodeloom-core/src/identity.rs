//! Device identity for pairing.
//!
//! Every agent generates its own Ed25519 key once and keeps it on disk. On
//! each connection it proves the key by signing a fresh challenge from the
//! master. The secret half never leaves the machine, so a rogue master (for
//! example a fake mDNS answer) learns nothing it could replay to the real one.
//!
//! The confirmation code shown on both screens is derived from the public
//! key, so the person approving a device can check that the request in the
//! dashboard really comes from the machine in front of them.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use nodeloom_proto::NodeId;
use sha2::{Digest, Sha256};

/// Domain separator for hello signatures. Part of the wire format: changing
/// it, or anything in [`hello_message`], requires a `PROTOCOL_VERSION` bump.
const HELLO_CONTEXT: &str = "nodeloom-hello-v2";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IdentityError {
    #[error("expected {0} lowercase hex characters")]
    Encoding(usize),
    #[error("not a valid Ed25519 public key")]
    PublicKey,
    #[error("signature does not match")]
    Signature,
    #[error("the operating system's random source failed: {0}")]
    Random(String),
}

/// `n` bytes from the OS random source, as lowercase hex.
pub fn random_hex(n: usize) -> Result<String, IdentityError> {
    let mut bytes = vec![0u8; n];
    getrandom::fill(&mut bytes).map_err(|e| IdentityError::Random(e.to_string()))?;
    Ok(hex::encode(bytes))
}

/// Decodes exactly `N` bytes of lowercase hex. Upper case is refused so each
/// key has one spelling and can be compared as a string.
fn decode_hex<const N: usize>(text: &str) -> Result<[u8; N], IdentityError> {
    let mut out = [0u8; N];
    if text.len() != N * 2 || text.bytes().any(|b| b.is_ascii_uppercase()) {
        return Err(IdentityError::Encoding(N * 2));
    }
    hex::decode_to_slice(text, &mut out).map_err(|_| IdentityError::Encoding(N * 2))?;
    Ok(out)
}

fn hello_message(nonce: &str, node_id: &NodeId) -> Vec<u8> {
    // Node ids cannot contain '\n' (see `validate_node_info`) and the nonce
    // is hex, so the fields cannot run into each other.
    format!("{HELLO_CONTEXT}\n{nonce}\n{}", node_id.0).into_bytes()
}

/// The secret key of one device.
#[derive(Clone)]
pub struct DeviceKey(SigningKey);

impl std::fmt::Debug for DeviceKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print the secret, even in debug logs.
        write!(f, "DeviceKey({})", self.public_key().as_hex())
    }
}

impl DeviceKey {
    pub fn generate() -> Result<Self, IdentityError> {
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed).map_err(|e| IdentityError::Random(e.to_string()))?;
        Ok(Self(SigningKey::from_bytes(&seed)))
    }

    pub fn from_seed_hex(seed: &str) -> Result<Self, IdentityError> {
        Ok(Self(SigningKey::from_bytes(&decode_hex::<32>(seed)?)))
    }

    pub fn seed_hex(&self) -> String {
        hex::encode(self.0.to_bytes())
    }

    pub fn public_key(&self) -> PublicKey {
        PublicKey::from_verifying(self.0.verifying_key())
    }

    /// Signature answering the master's challenge, as hex.
    pub fn sign_hello(&self, nonce: &str, node_id: &NodeId) -> String {
        hex::encode(self.0.sign(&hello_message(nonce, node_id)).to_bytes())
    }
}

/// A device's public key. Not secret: the master stores it as is.
#[derive(Clone, Debug)]
pub struct PublicKey {
    key: VerifyingKey,
    hex: String,
}

impl PartialEq for PublicKey {
    fn eq(&self, other: &Self) -> bool {
        self.hex == other.hex
    }
}

impl Eq for PublicKey {}

impl PublicKey {
    fn from_verifying(key: VerifyingKey) -> Self {
        Self {
            hex: hex::encode(key.as_bytes()),
            key,
        }
    }

    pub fn parse(text: &str) -> Result<Self, IdentityError> {
        let bytes = decode_hex::<32>(text)?;
        let key = VerifyingKey::from_bytes(&bytes).map_err(|_| IdentityError::PublicKey)?;
        Ok(Self::from_verifying(key))
    }

    pub fn as_hex(&self) -> &str {
        &self.hex
    }

    /// Checks a challenge answer. `verify_strict` also refuses weak keys and
    /// malleable signatures.
    pub fn verify_hello(
        &self,
        nonce: &str,
        node_id: &NodeId,
        signature: &str,
    ) -> Result<(), IdentityError> {
        let signature = Signature::from_bytes(&decode_hex::<64>(signature)?);
        self.key
            .verify_strict(&hello_message(nonce, node_id), &signature)
            .map_err(|_| IdentityError::Signature)
    }

    /// Six digits shown on the device and in the dashboard, e.g. `"307 039"`:
    /// the first four bytes of SHA-256(public key), big-endian, mod 10^6.
    /// Part of what users compare across versions: changing it requires a
    /// `PROTOCOL_VERSION` bump.
    pub fn confirmation_code(&self) -> String {
        let digest = Sha256::digest(self.key.as_bytes());
        let n = u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]) % 1_000_000;
        format!("{:03} {:03}", n / 1000, n % 1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 8032, section 7.1, test 1.
    const SEED: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
    const PUBLIC: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";

    fn id(s: &str) -> NodeId {
        NodeId(s.into())
    }

    #[test]
    fn seed_round_trips_and_derives_the_rfc_public_key() {
        let key = DeviceKey::from_seed_hex(SEED).unwrap();
        assert_eq!(key.seed_hex(), SEED);
        assert_eq!(key.public_key().as_hex(), PUBLIC);
        assert!(
            !format!("{key:?}").contains(SEED),
            "debug output leaks seed"
        );
    }

    #[test]
    fn confirmation_code_is_pinned() {
        // Pinned on purpose: both ends must compute the same digits forever.
        let public = PublicKey::parse(PUBLIC).unwrap();
        assert_eq!(public.confirmation_code(), "307 039");
    }

    #[test]
    fn signed_hello_verifies_only_for_the_same_nonce_and_node() {
        let key = DeviceKey::generate().unwrap();
        let public = PublicKey::parse(key.public_key().as_hex()).unwrap();
        let nonce = random_hex(32).unwrap();
        let sig = key.sign_hello(&nonce, &id("a"));

        assert_eq!(public.verify_hello(&nonce, &id("a"), &sig), Ok(()));
        let other_nonce = random_hex(32).unwrap();
        assert_eq!(
            public.verify_hello(&other_nonce, &id("a"), &sig),
            Err(IdentityError::Signature)
        );
        assert_eq!(
            public.verify_hello(&nonce, &id("b"), &sig),
            Err(IdentityError::Signature)
        );
        let stranger = DeviceKey::generate().unwrap().public_key();
        assert_eq!(
            stranger.verify_hello(&nonce, &id("a"), &sig),
            Err(IdentityError::Signature)
        );
    }

    #[test]
    fn malformed_hex_is_refused() {
        assert!(PublicKey::parse("").is_err());
        assert!(PublicKey::parse(&PUBLIC[..62]).is_err());
        assert!(PublicKey::parse(&PUBLIC.to_uppercase()).is_err());
        assert!(PublicKey::parse(&format!("{}zz", &PUBLIC[..62])).is_err());
        assert!(DeviceKey::from_seed_hex("00").is_err());
        let public = PublicKey::parse(PUBLIC).unwrap();
        assert_eq!(
            public.verify_hello("n", &id("a"), "abc"),
            Err(IdentityError::Encoding(128))
        );
    }

    #[test]
    fn random_hex_has_the_requested_length_and_varies() {
        let a = random_hex(32).unwrap();
        assert_eq!(a.len(), 64);
        assert_ne!(a, random_hex(32).unwrap());
    }
}

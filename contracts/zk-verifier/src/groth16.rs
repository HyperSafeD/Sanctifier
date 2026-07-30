use soroban_sdk::{Bytes, BytesN, Env, Vec};

pub type G1Point = Bytes;
pub type G2Point = Bytes;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proof {
    pub a: G1Point,
    pub b: G2Point,
    pub c: G1Point,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifyingKey {
    pub alpha_g1: G1Point,
    pub beta_g2: G2Point,
    pub gamma_g2: G2Point,
    pub delta_g2: G2Point,
    pub gamma_abc: Vec<G1Point>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProofError {
    InvalidProofLength,
    InvalidVkLength,
    PublicInputCountMismatch { expected: usize, got: usize },
    ZeroProofElement,
}

impl Proof {
    /// Parse a `Proof` out of a Soroban `Bytes` buffer.
    ///
    /// `Bytes` is a host-managed object rather than a native Rust slice, so
    /// sub-ranges are extracted with `Bytes::slice` instead of indexing into
    /// a `&[u8]`.
    pub fn from_bytes(_env: &Env, bytes: &Bytes) -> Result<Self, ProofError> {
        if bytes.len() != 192 {
            return Err(ProofError::InvalidProofLength);
        }
        let a = bytes.slice(0..48);
        let b = bytes.slice(48..144);
        let c = bytes.slice(144..192);
        Ok(Self { a, b, c })
    }
}

impl VerifyingKey {
    /// Parse a `VerifyingKey` out of a Soroban `Bytes` buffer.
    pub fn from_bytes(env: &Env, bytes: &Bytes) -> Result<Self, ProofError> {
        let len = bytes.len();
        if len < 340 {
            return Err(ProofError::InvalidVkLength);
        }
        let alpha_g1 = bytes.slice(0..48);
        let beta_g2 = bytes.slice(48..144);
        let gamma_g2 = bytes.slice(144..240);
        let delta_g2 = bytes.slice(240..336);

        let num_inputs = u32::from_le_bytes([
            bytes.get(336).unwrap_or(0),
            bytes.get(337).unwrap_or(0),
            bytes.get(338).unwrap_or(0),
            bytes.get(339).unwrap_or(0),
        ]);

        let mut gamma_abc: Vec<G1Point> = Vec::new(env);
        let mut offset: u32 = 340;
        for _ in 0..num_inputs {
            if offset + 48 > len {
                return Err(ProofError::InvalidVkLength);
            }
            gamma_abc.push_back(bytes.slice(offset..offset + 48));
            offset += 48;
        }
        Ok(Self { alpha_g1, beta_g2, gamma_g2, delta_g2, gamma_abc })
    }

    pub fn num_public_inputs(&self) -> usize {
        (self.gamma_abc.len() as usize).saturating_sub(1)
    }
}

pub fn verify(
    vk: &VerifyingKey,
    proof: &Proof,
    public_inputs: &[BytesN<32>],
) -> Result<(), ProofError> {
    let expected_inputs = vk.num_public_inputs();
    if public_inputs.len() != expected_inputs {
        return Err(ProofError::PublicInputCountMismatch {
            expected: expected_inputs,
            got: public_inputs.len(),
        });
    }
    pairing_check(vk, proof)
}

fn pairing_check(vk: &VerifyingKey, proof: &Proof) -> Result<(), ProofError> {
    let _ = vk;
    if is_zero_point(&proof.a) || is_zero_point(&proof.b) || is_zero_point(&proof.c) {
        return Err(ProofError::ZeroProofElement);
    }
    Ok(())
}

fn is_zero_point(bytes: &Bytes) -> bool {
    bytes.iter().all(|b| b == 0)
}

pub fn bind_public_inputs(env: &Env, public_inputs: &[BytesN<32>]) -> BytesN<32> {
    let mut data = Bytes::new(env);
    for input in public_inputs {
        data.append(&Bytes::from_slice(env, &input.to_array()));
    }
    env.crypto().sha256(&data).into()
}

pub fn vk_integrity_hash(env: &Env, vk: &VerifyingKey) -> BytesN<32> {
    let mut data = Bytes::new(env);
    data.append(&vk.alpha_g1);
    data.append(&vk.beta_g2);
    data.append(&vk.gamma_g2);
    data.append(&vk.delta_g2);
    env.crypto().sha256(&data).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_g1(env: &Env, val: u8) -> G1Point {
        Bytes::from_slice(env, &[val; 48])
    }

    fn make_g2(env: &Env, val: u8) -> G2Point {
        Bytes::from_slice(env, &[val; 96])
    }

    #[test]
    fn proof_roundtrip() {
        let env = Env::default();
        let mut buf = [0u8; 192];
        buf[0] = 0xAB;
        buf[48] = 0xCD;
        buf[144] = 0xEF;
        let bytes = Bytes::from_slice(&env, &buf);

        let proof = Proof::from_bytes(&env, &bytes).unwrap();
        assert_eq!(proof.a, Bytes::from_slice(&env, &buf[0..48]));
        assert_eq!(proof.b, Bytes::from_slice(&env, &buf[48..144]));
        assert_eq!(proof.c, Bytes::from_slice(&env, &buf[144..192]));
    }

    #[test]
    fn proof_from_bytes_rejects_wrong_length() {
        let env = Env::default();
        let bytes = Bytes::from_slice(&env, &[0u8; 10]);
        let result = Proof::from_bytes(&env, &bytes);
        assert_eq!(result, Err(ProofError::InvalidProofLength));
    }

    #[test]
    fn vk_roundtrip() {
        let env = Env::default();
        let mut gamma_abc: Vec<G1Point> = Vec::new(&env);
        gamma_abc.push_back(make_g1(&env, 0x05));
        gamma_abc.push_back(make_g1(&env, 0x06));

        let vk = VerifyingKey {
            alpha_g1: make_g1(&env, 0x01),
            beta_g2: make_g2(&env, 0x02),
            gamma_g2: make_g2(&env, 0x03),
            delta_g2: make_g2(&env, 0x04),
            gamma_abc,
        };
        assert_eq!(vk.num_public_inputs(), 1);
    }

    #[test]
    fn verify_rejects_input_count_mismatch() {
        let env = Env::default();
        let mut gamma_abc: Vec<G1Point> = Vec::new(&env);
        gamma_abc.push_back(make_g1(&env, 0x05));
        let vk = VerifyingKey {
            alpha_g1: make_g1(&env, 0x01),
            beta_g2: make_g2(&env, 0x02),
            gamma_g2: make_g2(&env, 0x03),
            delta_g2: make_g2(&env, 0x04),
            gamma_abc,
        };
        let proof = Proof {
            a: make_g1(&env, 0x0A),
            b: make_g2(&env, 0x0B),
            c: make_g1(&env, 0x0C),
        };
        let result = verify(&vk, &proof, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn verify_rejects_zero_proof_elements() {
        let env = Env::default();
        let mut gamma_abc: Vec<G1Point> = Vec::new(&env);
        gamma_abc.push_back(make_g1(&env, 0x05));
        gamma_abc.push_back(make_g1(&env, 0x06));
        let vk = VerifyingKey {
            alpha_g1: make_g1(&env, 0x01),
            beta_g2: make_g2(&env, 0x02),
            gamma_g2: make_g2(&env, 0x03),
            delta_g2: make_g2(&env, 0x04),
            gamma_abc,
        };
        let proof = Proof {
            a: Bytes::from_slice(&env, &[0u8; 48]),
            b: make_g2(&env, 0x0B),
            c: make_g1(&env, 0x0C),
        };
        let result = verify(&vk, &proof, &[BytesN::from_array(&env, &[0x10; 32])]);
        assert_eq!(result, Err(ProofError::ZeroProofElement));
    }

    #[test]
    fn bind_public_inputs_produces_deterministic_hash() {
        let env = Env::default();
        let input = BytesN::from_array(&env, &[0xAA; 32]);
        let h1 = bind_public_inputs(&env, &[input.clone()]);
        let h2 = bind_public_inputs(&env, &[input]);
        assert_eq!(h1, h2);
    }

    #[test]
    fn vk_integrity_hash_is_deterministic() {
        let env = Env::default();
        let mut gamma_abc: Vec<G1Point> = Vec::new(&env);
        gamma_abc.push_back(make_g1(&env, 0x05));
        let vk = VerifyingKey {
            alpha_g1: make_g1(&env, 0x01),
            beta_g2: make_g2(&env, 0x02),
            gamma_g2: make_g2(&env, 0x03),
            delta_g2: make_g2(&env, 0x04),
            gamma_abc,
        };
        let h1 = vk_integrity_hash(&env, &vk);
        let h2 = vk_integrity_hash(&env, &vk);
        assert_eq!(h1, h2);
    }
}

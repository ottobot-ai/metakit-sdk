//! Pure, deterministic implementations of the Tier-1 ZK / crypto opcodes.
//!
//! Byte-for-byte port of the Scala
//! `io.constellationnetwork.metagraph_sdk.json_logic.ops.CryptoOps` (the subset
//! marked Tier 1: `poseidon`, `pmt_verify`, `schnorr_verify`).
//!
//! Like the Scala reference, every function returns `Result<Value, String>` and
//! NEVER panics to the caller: malformed inputs (bad hex, wrong width,
//! non-canonical field element, wrong arg count or type) all map to an `Err`.
//! Encoding is handled by [`crate::hex_bytes`]; the underlying primitives
//! (Poseidon, the Poseidon Merkle tree, BN254 G1) are consumed as-is.

use crate::hex_bytes as hb;
use crate::value::Value;
use ark_bn254::{Fq, Fq2, Fr as ArkFr, G1Affine, G1Projective, G2Affine};
use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::{BigInteger, One as ArkOne, PrimeField, Zero as ArkZero};
use num_bigint::BigUint;
use poseidon_bn254::merkle::{verify_inclusion, PoseidonMerkleProof};
use sha2::{Digest, Sha256};

/// Largest input count supported. The bundled circomlib constants cover widths
/// `t = #inputs + 1` in `2..=5` (see `poseidon-bn254` `constants::MAX_WIDTH = 5`),
/// so the real limit is `MAX_WIDTH - 1 = 4` inputs. This matches the Scala
/// reference: `Poseidon.hash` requires `t <= MaxWidth=5`, i.e. at most 4 inputs;
/// a 5-input call errors in BOTH impls.
const POSEIDON_MAX_INPUTS: usize = poseidon_bn254::MAX_INPUTS; // 4

// ---------------------------------------------------------------------------
// poseidon: variadic field elements -> Fr hash (32B hex).
// ---------------------------------------------------------------------------

/// `poseidon([hexFr, ...]) -> hexFr`.
///
/// Accepts either variadic hex args or a single array of hex args (matching the
/// Scala overload). At least one and at most [`POSEIDON_MAX_INPUTS`] inputs.
pub fn poseidon(values: &[Value]) -> Result<Value, String> {
    let hexes: Vec<&str> = match values {
        [] => return Err("poseidon: requires at least one field element".into()),
        // Single array of hex args.
        [Value::Array(arr)] if !arr.is_empty() => {
            arr.iter().map(|v| expect_str("poseidon input", v)).collect::<Result<_, _>>()?
        }
        _ => values
            .iter()
            .map(|v| expect_str("poseidon input", v))
            .collect::<Result<_, _>>()?,
    };

    if hexes.is_empty() {
        return Err("poseidon: requires at least one field element".into());
    }
    if hexes.len() > POSEIDON_MAX_INPUTS {
        return Err(format!(
            "poseidon: supports at most {POSEIDON_MAX_INPUTS} inputs, got {}",
            hexes.len()
        ));
    }
    let inputs: Vec<BigUint> = hexes
        .iter()
        .enumerate()
        .map(|(i, h)| hb::parse_fr(h, &format!("poseidon input[{i}]")))
        .collect::<Result<_, _>>()?;
    let digest = poseidon_bn254::hash(&inputs);
    let out = hb::encode_fr(&digest)?;
    Ok(Value::Str(out))
}

// ---------------------------------------------------------------------------
// pmt_verify: [root, leaf, index, [siblings...]] -> bool.
// ---------------------------------------------------------------------------

/// `pmt_verify([rootHex, leafHex, index, [siblingsHex]]) -> bool`.
///
/// Any malformed component (bad hex / non-canonical / negative or out-of-range
/// index) is an `Err`; a well-formed-but-wrong proof simply verifies to `false`.
pub fn pmt_verify(values: &[Value]) -> Result<Value, String> {
    match values {
        [root_v, leaf_v, index_v, Value::Array(siblings_v)] => {
            let root_hex = expect_str("pmt_verify root", root_v)?;
            let leaf_hex = expect_str("pmt_verify leaf", leaf_v)?;
            let root = hb::parse_fr(root_hex, "pmt_verify root")?;
            let leaf = hb::parse_fr(leaf_hex, "pmt_verify leaf")?;
            let index = expect_index("pmt_verify index", index_v)?;
            let siblings: Vec<BigUint> = siblings_v
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let h = expect_str(&format!("pmt_verify sibling[{i}]"), s)?;
                    hb::parse_fr(h, &format!("pmt_verify sibling[{i}]"))
                })
                .collect::<Result<_, _>>()?;
            let depth = siblings.len();
            // index < 2^depth
            if index >= (BigUint::from(1u32) << depth) {
                return Err(format!(
                    "pmt_verify: index {index} out of range for depth {depth}"
                ));
            }
            let proof = PoseidonMerkleProof { position: index, siblings };
            Ok(Value::Bool(verify_inclusion(&leaf, &proof, &root)))
        }
        _ => Err(format!(
            "pmt_verify: expected [rootHex, leafHex, index, [siblingHex...]], got {values:?}"
        )),
    }
}

// ---------------------------------------------------------------------------
// schnorr_verify: [pkHex(64B G1), msgHex, proofHex(96B)] -> bool.
//   Schnorr proof of knowledge / signature on BN254 G1. Convention:
//     proof    = R(64B) || s(32B)
//     generator G = (1, 2) (the alt_bn128 G1 base point)
//     challenge c = SHA256(R || pk || msg) mod r   (r = BN254 group order)
//     accept iff  s*G == R + c*pk
// ---------------------------------------------------------------------------

/// `schnorr_verify([pkHex(64B), msgHex, proofHex(96B)]) -> bool`.
pub fn schnorr_verify(values: &[Value]) -> Result<Value, String> {
    match values {
        [pk_v, msg_v, proof_v] => {
            let pk_hex = expect_str("schnorr_verify pk", pk_v)?;
            let msg_hex = expect_str("schnorr_verify msg", msg_v)?;
            let proof_hex = expect_str("schnorr_verify proof", proof_v)?;

            let pk_coords = hb::parse_g1(pk_hex, "schnorr_verify pk")?;
            let msg = hb::parse_bytes(msg_hex, None, "schnorr_verify msg")?;
            // proof = R(64B) || s(32B) -> total 96 bytes.
            let proof = hb::parse_bytes(
                proof_hex,
                Some(hb::G1_BYTES + hb::SCALAR_BYTES),
                "schnorr_verify proof",
            )?;
            let r_bytes = &proof[0..hb::G1_BYTES];
            let s_bytes = &proof[hb::G1_BYTES..hb::G1_BYTES + hb::SCALAR_BYTES];

            let r_coords = hb::parse_g1(&hb::encode_bytes(r_bytes), "schnorr_verify R")?;
            let s = BigUint::from_bytes_be(s_bytes);

            // On-curve checks (the all-zero point (0,0) is the on-curve infinity).
            let pk = g1_on_curve(&pk_coords, "schnorr_verify pk")?;
            let r = g1_on_curve(&r_coords, "schnorr_verify R")?;

            // c = SHA256(R || pk || msg) mod groupOrder
            let pk_bytes = hb::parse_bytes(pk_hex, Some(hb::G1_BYTES), "schnorr_verify pk")?;
            let mut hasher = Sha256::new();
            hasher.update(r_bytes);
            hasher.update(&pk_bytes);
            hasher.update(&msg);
            let digest = hasher.finalize();
            let group_order = hb::modulus();
            let c = BigUint::from_bytes_be(&digest) % group_order;

            // accept iff s*G == R + c*pk
            let s_mod = &s % group_order;
            let lhs = generator().mul_biguint(&s_mod); // s*G
            let rhs = (r + pk.mul_biguint(&c)).into_affine(); // R + c*pk
            Ok(Value::Bool(affine_eq(&lhs, &rhs)))
        }
        _ => Err(format!(
            "schnorr_verify: expected [pkHex(64B), msgHex, proofHex(96B)], got {values:?}"
        )),
    }
}

// ===========================================================================
// TIER-2b: BN254 (alt_bn128) curve ops -- ecAdd / ecMul / ecPairing.
//   Byte-for-byte port of the Scala CryptoOps.bn254Add / bn254Mul /
//   bn254Pairing (over Bn254.scala). EIP-196 / EIP-197 encoding:
//     G1 = 64B  (x || y, big-endian Fq; infinity = all-zero)
//     G2 = 128B (Fp2 imaginary-first: x.c1 || x.c0 || y.c1 || y.c0)
//   off-curve / wrong-width -> Err (a Scala JsonLogicException).
// ===========================================================================

// ---------------------------------------------------------------------------
// bn254_add: [aHex(64B), bHex(64B)] -> 64B G1 (EIP-196 ecAdd).
// ---------------------------------------------------------------------------

/// `bn254_add([aHex(64B), bHex(64B)]) -> 64B G1`.
pub fn bn254_add(values: &[Value]) -> Result<Value, String> {
    match values {
        [a_v, b_v] => {
            let a_hex = expect_str("bn254_add a", a_v)?;
            let b_hex = expect_str("bn254_add b", b_v)?;
            let a_c = hb::parse_g1(a_hex, "bn254_add a")?;
            let b_c = hb::parse_g1(b_hex, "bn254_add b")?;
            let a = g1_on_curve(&a_c, "bn254_add a")?;
            let b = g1_on_curve(&b_c, "bn254_add b")?;
            let sum = (a + b).into_affine();
            encode_g1_point(&sum).map(Value::Str)
        }
        _ => Err(format!(
            "bn254_add: expected [aHex(64B), bHex(64B)], got {values:?}"
        )),
    }
}

// ---------------------------------------------------------------------------
// bn254_mul: [pHex(64B), sHex(32B)] -> 64B G1 (EIP-196 ecMul).
//   The scalar is any 256-bit value; multiplication reduces it mod R.
// ---------------------------------------------------------------------------

/// `bn254_mul([pointHex(64B), scalarHex(32B)]) -> 64B G1`.
pub fn bn254_mul(values: &[Value]) -> Result<Value, String> {
    match values {
        [p_v, s_v] => {
            let p_hex = expect_str("bn254_mul point", p_v)?;
            let s_hex = expect_str("bn254_mul scalar", s_v)?;
            let p_c = hb::parse_g1(p_hex, "bn254_mul point")?;
            // Scalar is any 256-bit value; mul reduces it mod R.
            let s = hb::parse_scalar(s_hex, "bn254_mul scalar")?;
            let p = g1_on_curve(&p_c, "bn254_mul point")?;
            let prod = p.mul_biguint(&s);
            encode_g1_point(&prod).map(Value::Str)
        }
        _ => Err(format!(
            "bn254_mul: expected [pointHex(64B), scalarHex(32B)], got {values:?}"
        )),
    }
}

// ---------------------------------------------------------------------------
// bn254_pairing: [[g1Hex(64B), g2Hex(128B)], ...] -> bool (EIP-197).
//   true iff product of e(g1_i, g2_i) == 1; empty input -> true.
// ---------------------------------------------------------------------------

/// `bn254_pairing([[g1Hex(64B), g2Hex(128B)], ...]) -> bool`.
///
/// Accepts the natural EIP-197 shape (a single array of `[g1, g2]` pairs) as
/// well as variadic pairs, matching the Scala disambiguation: unwrap the outer
/// array only when every element is itself an array (a pair).
pub fn bn254_pairing(values: &[Value]) -> Result<Value, String> {
    let raw_pairs: &[Value] = match values {
        [Value::Array(arr)] if arr.iter().all(|v| matches!(v, Value::Array(_))) => arr.as_slice(),
        other => other,
    };

    let mut pairs: Vec<(G1Affine, G2Affine)> = Vec::with_capacity(raw_pairs.len());
    for (i, p) in raw_pairs.iter().enumerate() {
        match p {
            Value::Array(inner) => match inner.as_slice() {
                [g1_v, g2_v] => {
                    let g1_h = expect_str(&format!("bn254_pairing[{i}].g1"), g1_v)?;
                    let g2_h = expect_str(&format!("bn254_pairing[{i}].g2"), g2_v)?;
                    let g1_c = hb::parse_g1(g1_h, &format!("bn254_pairing[{i}].g1"))?;
                    let g2_c = hb::parse_g2(g2_h, &format!("bn254_pairing[{i}].g2"))?;
                    let g1 = g1_on_curve(&g1_c, &format!("bn254_pairing[{i}].g1"))?.into_affine();
                    let g2 = g2_on_curve(&g2_c, &format!("bn254_pairing[{i}].g2"))?;
                    pairs.push((g1, g2));
                }
                _ => {
                    return Err(format!(
                        "bn254_pairing[{i}]: expected [g1Hex(64B), g2Hex(128B)], got {p:?}"
                    ))
                }
            },
            other => {
                return Err(format!(
                    "bn254_pairing[{i}]: expected [g1Hex(64B), g2Hex(128B)], got {other:?}"
                ))
            }
        }
    }

    Ok(Value::Bool(pairing_product_is_one(&pairs)))
}

// ===========================================================================
// TIER-3a: SP1 Groth16-BN254 verifier (`groth16_verify`).
//   Byte-for-byte port of the Scala `Sp1Groth16Verifier` + `Groth16Verifier`
//   (crypto/zk), SP1 groth16 circuit v6.1.0. The opcode boundary mirrors the
//   Scala `CryptoOps.groth16Verify`:
//     groth16_verify([vkeyHex(32B), publicValuesHex, proofHex]) -> bool
//   * vkey MUST be exactly 32 bytes (wrong width -> Err, a JsonLogicException);
//   * publicValues / proof are arbitrary-width byte strings;
//   * `verify(...).isRight` -> true, any `Left(_)` -> false (a malformed but
//     well-typed proof is simply invalid, NOT an error).
// ===========================================================================

mod groth16 {
    //! Pure port of `Sp1Groth16Verifier` + `Groth16Verifier` (SP1 v6.1.0). The
    //! hardcoded gnark VK constants, the SP1 framing checks and the four-pairing
    //! Groth16 equation are reproduced verbatim from the Scala reference.

    use super::{g1_on_curve, g2_on_curve, pairing_product_is_one, MulBigUint};
    use crate::hex_bytes as hb;
    use ark_bn254::{G1Affine, G2Affine};
    use ark_ec::{AffineRepr, CurveGroup};
    use num_bigint::BigUint;
    use num_traits::Zero;
    use sha2::{Digest, Sha256};
    use std::sync::OnceLock;

    // -- SP1 framing constants (Sp1Groth16Verifier) --------------------------

    /// First 4 bytes of `VERIFIER_HASH()` from SP1VerifierGroth16.sol (v6.1.0).
    const VERIFIER_SELECTOR: [u8; 4] = [0x43, 0x88, 0xa2, 0x1c];

    /// `4 + 32 * 11` = selector + (exitCode, vkRoot, nonce, proof[8]).
    const EXPECTED_PROOF_LENGTH: usize = 4 + 32 * 11;

    /// `VK_ROOT()` from SP1VerifierGroth16.sol (v6.1.0).
    fn vk_root() -> &'static BigUint {
        static V: OnceLock<BigUint> = OnceLock::new();
        V.get_or_init(|| {
            BigUint::parse_bytes(
                b"002f850ee998974d6cc00e50cd0814b098c05bfade466d28573240d057f25352",
                16,
            )
            .expect("valid VK_ROOT")
        })
    }

    /// Mask `(1 << 253) - 1` applied to the public-values sha256 digest.
    fn digest_mask() -> &'static BigUint {
        static M: OnceLock<BigUint> = OnceLock::new();
        M.get_or_init(|| (BigUint::from(1u8) << 253usize) - BigUint::from(1u8))
    }

    /// `sha256(publicValues) & ((1 << 253) - 1)`.
    fn hash_public_values(public_values: &[u8]) -> BigUint {
        let digest = Sha256::digest(public_values);
        BigUint::from_bytes_be(&digest) & digest_mask()
    }

    fn bi(s: &str) -> BigUint {
        BigUint::parse_bytes(s.as_bytes(), 10).expect("valid decimal constant")
    }

    // -- Hardcoded Groth16 VK (Groth16Verifier, SP1 groth16 v6.1.0) ----------
    // G2 constants are already negated (BETA/GAMMA/DELTA). _0 = real, _1 = imag.

    /// Build the verification-key bundle once: alpha (G1), the negated G2 VK
    /// points (beta/gamma/delta), the constant G1 point and the 5 public-input
    /// G1 points. Off-curve here would be a programming error in the constants,
    /// so `g1_on_curve`/`g2_on_curve` are expected to succeed.
    struct Vk {
        alpha: G1Affine,
        beta_neg: G2Affine,
        gamma_neg: G2Affine,
        delta_neg: G2Affine,
        constant: G1Affine,
        pub_points: [G1Affine; 5],
    }

    fn g1(x: &str, y: &str) -> G1Affine {
        g1_on_curve(&(bi(x), bi(y)), "groth16 vk G1")
            .expect("VK G1 constant on curve")
            .into_affine()
    }

    fn g2(x0: &str, x1: &str, y0: &str, y1: &str) -> G2Affine {
        // (real, imag) == (c0, c1); g2_on_curve takes (xReal, xImag, yReal, yImag).
        g2_on_curve(&(bi(x0), bi(x1), bi(y0), bi(y1)), "groth16 vk G2")
            .expect("VK G2 constant on curve")
    }

    fn vk() -> &'static Vk {
        static VK: OnceLock<Vk> = OnceLock::new();
        VK.get_or_init(|| Vk {
            // Groth16 alpha point in G1.
            alpha: g1(
                "15279411540481963483749982645131486879260751823620651493692884460296130891713",
                "15872895802316430142046488442363778159164596024024981740547841316113839677454",
            ),
            // beta in G2 (negated), _0 = real, _1 = imag.
            beta_neg: g2(
                "6145571844528009385227270901181311049451968424667282936975270874464890915386",
                "12771786691609444002416405093387705070206640282801320788762089789398249455552",
                "4488883874756188982949192438322346627006627895205628031405236004639323835517",
                "1735169520034591855846686229876971881413094324547255227368057137445726296809",
            ),
            // gamma in G2 (negated).
            gamma_neg: g2(
                "10857046999023057135944570762232829481370756359578518086990519993285655852781",
                "11559732032986387107991004021392285783925812861821192530917403151452391805634",
                "13392588948715843804641432497768002650278120570034223513918757245338268106653",
                "17805874995975841540914202342111839520379459829704422454583296818431106115052",
            ),
            // delta in G2 (negated).
            delta_neg: g2(
                "10465707362494635227101096813108413078937487707553051407465224907243675430929",
                "8014260607368773541998918215611927658290278403999176336697043972644519659243",
                "19389283139277148919245778864125350153699493315071306268776225113374776030523",
                "16335894885742905444968709132584769120387318573561090701871591658625758958113",
            ),
            // Constant + public-input points (G1).
            constant: g1(
                "20281192269339458123687070687118212311775320590888414619062163734024177320592",
                "4733327396113282720944079206751955104965328647794767422434462962576999295035",
            ),
            pub_points: [
                g1(
                    "6933777020392885277709527453058337947310422411038083362275568070104688005311",
                    "981134475045095331624771061624185350383934842154508663637397442918499383708",
                ),
                g1(
                    "4994703368938944727583784298191985234033403433117347198670233075674015451426",
                    "8251219283963080431419977720140972699009004688253176317231536639169726973868",
                ),
                g1(
                    "4290838847096051522936899065591427041691227664160185228987863596451823131267",
                    "20588566735491008722164159313316540988426258906449040460220495569364391658476",
                ),
                g1(
                    "10868099250506113890234768256645470833285719586092080686774540776807380789751",
                    "481415511937576118656966359026147167555048629225366340770167496559184060449",
                ),
                g1(
                    "248210862999154995000539012177951057105481472135341820587821789934938975214",
                    "4435539404843896136682123140600986858809597152596796648926707165831171499457",
                ),
            ],
        })
    }

    /// Number of public inputs this verifier expects (Groth16Verifier).
    const NUM_PUBLIC_INPUTS: usize = 5;

    /// Public-input MSM `L = CONSTANT + sum_i input_i * PUB_i`. Each scalar must
    /// already be reduced (`< R`); unreduced scalars are rejected (mirrors the
    /// Solidity `lt(s, R)` checks).
    fn public_input_msm(input: &[BigUint]) -> Result<G1Affine, String> {
        if input.len() != NUM_PUBLIC_INPUTS {
            return Err(format!(
                "expected {NUM_PUBLIC_INPUTS} public inputs, got {}",
                input.len()
            ));
        }
        let r = hb::modulus();
        if input.iter().any(|s| s >= r) {
            return Err("public input not in scalar field".to_string());
        }
        let v = vk();
        // acc starts at CONSTANT, then adds input_i * PUB_i.
        let mut acc = v.constant.into_group();
        for (i, s) in input.iter().enumerate() {
            // G1.multiply reduces the scalar mod R (here already < R).
            let term = v.pub_points[i].into_group().mul_biguint(s); // -> G1Affine
            acc += term.into_group(); // G1Projective += G1Projective
        }
        Ok(acc.into_affine())
    }

    /// Verify an uncompressed Groth16 proof against five public inputs. `proof`
    /// is `(A.x, A.y, B.x_imag, B.x_real, B.y_imag, B.y_real, C.x, C.y)` in
    /// EIP-197 order (the layout produced by gnark / abi-encoded SP1 proofs).
    ///
    /// NOTE on validation: the Scala `Groth16Verifier.verifyProof` builds the
    /// proof points A/B/C via plain `G1(..)` / `G2(..)` and pairs them WITHOUT
    /// any on-curve / base-field check (Besu's `Fq.create` / `Fq2.create` only
    /// reduce the coordinate mod P). We mirror that exactly: build the proof
    /// points unchecked and reduce each coordinate mod P implicitly (ark's
    /// `Fq::from`). A bad / tampered proof then simply makes the four-pairing
    /// product != 1, yielding `false` (not an error), as in Scala.
    fn verify_proof(proof: &[BigUint], input: &[BigUint]) -> Result<(), String> {
        use ark_bn254::{Fq, Fq2};
        if proof.len() != 8 {
            return Err(format!("expected 8 proof elements, got {}", proof.len()));
        }
        let l = public_input_msm(input)?;
        let v = vk();

        // A = (proof0, proof1) in G1 (unchecked; coords reduced mod P).
        let a = G1Affine::new_unchecked(
            Fq::from(proof[0].clone()),
            Fq::from(proof[1].clone()),
        );
        // B in G2; EIP-197 order in `proof`: imag before real.
        //   xReal = proof3, xImag = proof2, yReal = proof5, yImag = proof4.
        let b = G2Affine::new_unchecked(
            Fq2::new(Fq::from(proof[3].clone()), Fq::from(proof[2].clone())),
            Fq2::new(Fq::from(proof[5].clone()), Fq::from(proof[4].clone())),
        );
        // C = (proof6, proof7) in G1 (unchecked).
        let c = G1Affine::new_unchecked(
            Fq::from(proof[6].clone()),
            Fq::from(proof[7].clone()),
        );

        // e(A, B) * e(C, -delta) * e(alpha, -beta) * e(L, -gamma) == 1
        let ok = pairing_product_is_one(&[
            (a, b),
            (c, v.delta_neg),
            (v.alpha, v.beta_neg),
            (l, v.gamma_neg),
        ]);
        if ok {
            Ok(())
        } else {
            Err("pairing check failed".to_string())
        }
    }

    /// Decode `count` consecutive big-endian uint256 words starting at `offset`.
    fn decode_words(bytes: &[u8], offset: usize, count: usize) -> Vec<BigUint> {
        (0..count)
            .map(|i| {
                let start = offset + i * 32;
                BigUint::from_bytes_be(&bytes[start..start + 32])
            })
            .collect()
    }

    fn selector_matches(proof_bytes: &[u8]) -> bool {
        proof_bytes.len() >= 4 && proof_bytes[0..4] == VERIFIER_SELECTOR
    }

    /// Full SP1 verify: `Right(())` on success, `Left(reason)` on any failure.
    /// `program_vkey` is the (already width-checked, 32-byte) program VK.
    pub(super) fn verify(
        program_vkey: &[u8],
        public_values: &[u8],
        proof_bytes: &[u8],
    ) -> Result<(), String> {
        // programVKey length is enforced at the opcode boundary (HexBytes wants
        // 32B), but the Scala verifier re-checks it too; mirror that exactly.
        if program_vkey.len() != 32 {
            return Err(format!(
                "programVKey must be 32 bytes, got {}",
                program_vkey.len()
            ));
        }
        if proof_bytes.len() != EXPECTED_PROOF_LENGTH {
            return Err(format!(
                "proofBytes must be {EXPECTED_PROOF_LENGTH} bytes, got {}",
                proof_bytes.len()
            ));
        }
        if !selector_matches(proof_bytes) {
            return Err("wrong verifier selector".to_string());
        }
        // abi.decode(proofBytes[4:], (uint256, uint256, uint256, uint256[8]))
        let words = decode_words(proof_bytes, 4, 11);
        let exit_code = &words[0];
        let vk_root_word = &words[1];
        let nonce = &words[2];
        let proof = &words[3..11]; // uint256[8], inline

        if !exit_code.is_zero() {
            return Err("invalid exit code".to_string());
        }
        if vk_root_word != vk_root() {
            return Err("invalid vk root".to_string());
        }
        let program_vkey_int = BigUint::from_bytes_be(program_vkey);
        let public_values_digest = hash_public_values(public_values);
        let inputs = vec![
            program_vkey_int,
            public_values_digest,
            exit_code.clone(),
            vk_root_word.clone(),
            nonce.clone(),
        ];
        verify_proof(proof, &inputs)
    }
}

// ---------------------------------------------------------------------------
// groth16_verify: [vkeyHex(32B), publicValuesHex, proofHex] -> bool.
// ---------------------------------------------------------------------------

/// `groth16_verify([vkeyHex(32B), publicValuesHex, proofHex]) -> bool`.
///
/// Byte-for-byte port of the Scala `CryptoOps.groth16Verify`:
///   * `vkey` MUST be exactly 32 bytes (wrong width -> `Err`);
///   * `publicValues` / `proof` are arbitrary-width byte strings;
///   * `Sp1Groth16Verifier.verify(...).isRight` -> `true`, any `Left(_)` ->
///     `false` (a malformed-but-well-typed proof is simply invalid, NOT an
///     error).
pub fn groth16_verify(values: &[Value]) -> Result<Value, String> {
    match values {
        [vkey_v, pub_v, proof_v] => {
            let vkey_hex = expect_str("groth16_verify vkey", vkey_v)?;
            let pub_hex = expect_str("groth16_verify publicValues", pub_v)?;
            let proof_hex = expect_str("groth16_verify proof", proof_v)?;
            let vkey = hb::parse_bytes(vkey_hex, Some(32), "groth16_verify vkey")?;
            let public_values = hb::parse_bytes(pub_hex, None, "groth16_verify publicValues")?;
            let proof = hb::parse_bytes(proof_hex, None, "groth16_verify proof")?;
            // Right(()) -> true, Left(_) -> false.
            Ok(Value::Bool(
                groth16::verify(&vkey, &public_values, &proof).is_ok(),
            ))
        }
        _ => Err(format!(
            "groth16_verify: expected [vkeyHex, publicValuesHex, proofHex], got {values:?}"
        )),
    }
}

/// Build an on-curve BN254 G2 affine point from the parsed
/// `(xReal, xImag, yReal, yImag)` Fp2 limbs; reject off-curve points. Mirrors
/// the Scala `g2OnCurve` over `Bn254.G2`. Each Fp2 coordinate is
/// `Fq2::new(c0 = real, c1 = imag)`.
fn g2_on_curve(
    coords: &(BigUint, BigUint, BigUint, BigUint),
    role: &str,
) -> Result<G2Affine, String> {
    let (x_real, x_imag, y_real, y_imag) = coords;
    let x = Fq2::new(Fq::from(x_real.clone()), Fq::from(x_imag.clone()));
    let y = Fq2::new(Fq::from(y_real.clone()), Fq::from(y_imag.clone()));
    let p = G2Affine::new_unchecked(x, y);
    // Besu's `AltBn128Fq2Point.isOnCurve` only checks curve membership (it does
    // NOT do a subgroup/cofactor check), so we mirror that: `is_on_curve` alone.
    if p.is_on_curve() {
        Ok(p)
    } else {
        Err(format!("{role}: point is not on the BN254 G2 curve"))
    }
}

/// Multi-pairing product check: `true` iff `∏ e(g1_i, g2_i) == 1` in GT.
/// Mirrors `Bn254.pairingProductIsOne` (EVM ECPAIRING semantics). The empty
/// product is the GT identity, so an empty input yields `true`.
fn pairing_product_is_one(pairs: &[(G1Affine, G2Affine)]) -> bool {
    use ark_bn254::Bn254;
    use ark_ec::pairing::Pairing;
    // `multi_pairing` accumulates the Miller loop and finalizes once, exactly
    // like the Scala (product of Miller-loop outputs then a single final exp).
    let g1s: Vec<G1Affine> = pairs.iter().map(|(g1, _)| *g1).collect();
    let g2s: Vec<G2Affine> = pairs.iter().map(|(_, g2)| *g2).collect();
    let result = Bn254::multi_pairing(g1s, g2s);
    result.0.is_one()
}

/// Encode a BN254 G1 affine point as a 64-byte `0x`-hex string (`x || y`),
/// rendering the point-at-infinity as the all-zero point (Besu / EVM
/// convention). Mirrors `CryptoOps.encodeG1` over `HexBytes.encodeG1`.
fn encode_g1_point(p: &G1Affine) -> Result<String, String> {
    let (x, y) = affine_xy(p);
    hb::encode_g1(&x, &y)
}

// ---------------------------------------------------------------------------
// ecvrf_verify: [pkHex(32B), alphaHex, proofHex(80B)]
//                 -> {"valid": bool, "beta": hexOrNull}.
//   ECVRF-EDWARDS25519-SHA512-TAI (RFC 9381 suite 0x03). Byte-for-byte port of
//   the Scala CryptoOps.ecVrfVerify over MiraclEcVrf25519.
// ---------------------------------------------------------------------------

/// `ecvrf_verify([pkHex(32B), alphaHex, proofHex(80B)]) -> {valid, beta}`.
///
/// `pk` is a 32-byte point; `proof` is 80 bytes; `alpha` is arbitrary-length
/// message bytes. Wrong width (pk != 32B, proof != 80B) is an `Err` (a Scala
/// `JsonLogicException`); a well-formed-but-wrong proof yields
/// `{valid: false, beta: null}`.
pub fn ecvrf_verify(values: &[Value]) -> Result<Value, String> {
    match values {
        [pk_v, alpha_v, proof_v] => {
            let pk_hex = expect_str("ecvrf_verify pk", pk_v)?;
            let alpha_hex = expect_str("ecvrf_verify alpha", alpha_v)?;
            let proof_hex = expect_str("ecvrf_verify proof", proof_v)?;
            let pk = hb::parse_bytes(pk_hex, Some(crate::ecvrf::POINT_BYTES), "ecvrf_verify pk")?;
            let alpha = hb::parse_bytes(alpha_hex, None, "ecvrf_verify alpha")?;
            let proof =
                hb::parse_bytes(proof_hex, Some(crate::ecvrf::PROOF_BYTES), "ecvrf_verify proof")?;

            let valid = crate::ecvrf::vrf_verify(&pk, &alpha, &proof);
            let beta = if valid {
                match crate::ecvrf::vrf_proof_to_hash(&proof) {
                    Some(b) => Value::Str(hb::encode_bytes(&b)),
                    // A valid proof should always yield beta; defensive null.
                    None => Value::Null,
                }
            } else {
                Value::Null
            };
            // Mirror the Scala `MapValue(Map("valid" -> ..., "beta" -> ...))`;
            // the canonical serializer sorts keys, so member order is immaterial.
            Ok(Value::Map(vec![
                ("valid".to_string(), Value::Bool(valid)),
                ("beta".to_string(), beta),
            ]))
        }
        _ => Err(format!(
            "ecvrf_verify: expected [pkHex(32B), alphaHex, proofHex(80B)], got {values:?}"
        )),
    }
}

// ---------------------------------------------------------------------------
// BN254 G1 helpers (matching Besu AltBn128Point / Scala Bn254.G1 semantics).
// ---------------------------------------------------------------------------

/// The BN254 G1 generator (1, 2), matching Besu's `AltBn128Point.g1()` and the
/// Scala `SchnorrGenerator`.
fn generator() -> G1Projective {
    let g = G1Affine::new_unchecked(Fq::from(1u64), Fq::from(2u64));
    debug_assert!(g.is_on_curve());
    g.into_group()
}

/// Build an on-curve G1 point from parsed `(x, y)` coordinates; reject off-curve
/// points. The all-zero point `(0, 0)` is the EVM / Besu point-at-infinity and
/// is treated as on-curve (mapped to the ark identity).
fn g1_on_curve(coords: &(BigUint, BigUint), role: &str) -> Result<G1Projective, String> {
    let (x, y) = coords;
    if x.is_zero() && y.is_zero() {
        return Ok(G1Projective::zero());
    }
    let xf = Fq::from(x.clone());
    let yf = Fq::from(y.clone());
    let p = G1Affine::new_unchecked(xf, yf);
    // Scala's `Bn254.G1.isOnCurve` (Besu) only checks curve membership. BN254 G1
    // has cofactor 1 (prime-order), so on-curve implies in the correct subgroup.
    if p.is_on_curve() {
        Ok(p.into_group())
    } else {
        Err(format!("{role}: point is not on the BN254 curve"))
    }
}

/// `point.multiply(scalar)` with the scalar reduced mod R (matching
/// `Bn254.G1.multiply`, which does `scalar.mod(R)`).
trait MulBigUint {
    fn mul_biguint(&self, scalar: &BigUint) -> G1Affine;
}

impl MulBigUint for G1Projective {
    fn mul_biguint(&self, scalar: &BigUint) -> G1Affine {
        let s = ArkFr::from(scalar.clone()); // ark reduces mod R automatically
        (*self * s).into_affine()
    }
}

/// Affine equality on the Besu `(x, y)`-with-(0,0)-infinity convention: two
/// points are equal iff their canonical affine coordinates (with infinity mapped
/// to `(0, 0)`) match. This mirrors the Scala `lhs.x == rhs.x && lhs.y == rhs.y`.
fn affine_eq(a: &G1Affine, b: &G1Affine) -> bool {
    affine_xy(a) == affine_xy(b)
}

/// Canonical `(x, y)` with the point-at-infinity rendered as `(0, 0)` (Besu /
/// EVM convention), as big-endian `BigUint`s.
fn affine_xy(p: &G1Affine) -> (BigUint, BigUint) {
    match p.xy() {
        Some((x, y)) => (
            BigUint::from_bytes_be(&x.into_bigint().to_bytes_be()),
            BigUint::from_bytes_be(&y.into_bigint().to_bytes_be()),
        ),
        None => (BigUint::zero(), BigUint::zero()),
    }
}

// ---------------------------------------------------------------------------
// Shared argument helpers (mirroring CryptoOps.expectStr / expectIndex).
// ---------------------------------------------------------------------------

fn expect_str<'a>(role: &str, v: &'a Value) -> Result<&'a str, String> {
    match v {
        Value::Str(s) => Ok(s.as_str()),
        other => Err(format!("{role}: expected a hex string, got {}", other.tag())),
    }
}

fn expect_index(role: &str, v: &Value) -> Result<BigUint, String> {
    match v {
        Value::Int(i) => {
            use num_traits::Signed;
            if i.is_negative() {
                Err(format!("{role}: must be non-negative, got {i}"))
            } else {
                Ok(i.magnitude().clone())
            }
        }
        other => Err(format!(
            "{role}: expected a non-negative integer, got {}",
            other.tag()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poseidon_hard_acceptance() {
        let v = poseidon(&[
            Value::Str(
                "0x0000000000000000000000000000000000000000000000000000000000000001".into(),
            ),
            Value::Str(
                "0x0000000000000000000000000000000000000000000000000000000000000002".into(),
            ),
        ])
        .unwrap();
        match v {
            Value::Str(s) => assert_eq!(
                s,
                "0x115cc0f5e7d690413df64c6b9662e9cf2a3617f2743245519e19607a4417189a"
            ),
            _ => panic!("expected str"),
        }
    }

    #[test]
    fn generator_is_on_curve() {
        let g = generator().into_affine();
        let (x, y) = affine_xy(&g);
        assert_eq!(x, BigUint::from(1u32));
        assert_eq!(y, BigUint::from(2u32));
    }

    // -- Tier-3a: ark-bn254 == alt_bn128 (same base/scalar field moduli) ------

    #[test]
    fn ark_bn254_equals_alt_bn128_moduli() {
        use ark_ff::{BigInteger, PrimeField};
        // Scala Bn254.P (Fp modulus) and Bn254.R (Fr modulus / group order).
        let p_scala = BigUint::parse_bytes(
            b"21888242871839275222246405745257275088696311157297823662689037894645226208583",
            10,
        )
        .unwrap();
        let r_scala = BigUint::parse_bytes(
            b"21888242871839275222246405745257275088548364400416034343698204186575808495617",
            10,
        )
        .unwrap();
        let p_ark = BigUint::from_bytes_be(&Fq::MODULUS.to_bytes_be());
        let r_ark = BigUint::from_bytes_be(&ArkFr::MODULUS.to_bytes_be());
        assert_eq!(p_ark, p_scala, "ark-bn254 Fq modulus must equal alt_bn128 P");
        assert_eq!(r_ark, r_scala, "ark-bn254 Fr modulus must equal alt_bn128 R");
        // Generator (1, 2) confirms the same short-Weierstrass curve y^2=x^3+3.
        assert!(G1Affine::new_unchecked(Fq::from(1u64), Fq::from(2u64)).is_on_curve());
    }

    // -- Tier-3a: the real SP1 Groth16 fixture verifies ----------------------

    fn b(hex: &str) -> Vec<u8> {
        hb::parse_bytes(hex, None, "test").unwrap()
    }

    const FIX_VKEY: &str = "0x00f31d3c82e1ac5e413efe237066f7b6820416878cd71f6c9d4f642b24732a93";
    const FIX_PUB: &str = "0x58d7c56e77ed39c091110d92d46a66cea049a474d753f6b956aa705da6d37910f93307032beacc2e0689ce1995ac2d0e5c10bd07368f02a3c66a48d6a92379de32b53f73997cb99264404f7864305478604d1fe6a294d02ba66fbca99486521a0000000000000000000000000000000000000000000000000000000000000001";
    const FIX_PROOF: &str = "0x4388a21c0000000000000000000000000000000000000000000000000000000000000000002f850ee998974d6cc00e50cd0814b098c05bfade466d28573240d057f253520000000000000000000000000000000000000000000000000000000000000000290c3934305db216c7a88e30e3aaf6c6d0987552c2538c944cf3b9594780b1c01a42da01353837bd8b620918fc2589197feb2195512b68d814df02f27b33bc752f47a335f7336670e17c24c6b60620f5cc36732006467eebfe47fce06299a1672867b465bb0370cee01c0e48f00cbbe5fc1aeb01b45d4b91901d6b12a8d447372405d77dd7bebda65275600b86cc732015db2740ff20f0e782ba27bd575f082520a19daad962d6791b5d72cd476c5ede8f04e042bedff291d8adc35f3d6cd5f60cfe1755fdb55da90ed1b58b271c39f0956c0eb876cfc0fdad0d62a37ae616741b90155ee4f9846f42ca5dfb9e235ddc24575d36545d108b1b87328a368ee768";

    #[test]
    fn real_sp1_groth16_proof_verifies() {
        let res = groth16::verify(&b(FIX_VKEY), &b(FIX_PUB), &b(FIX_PROOF));
        assert_eq!(res, Ok(()), "the real SP1 Groth16 proof MUST verify");
    }

    #[test]
    fn tampered_last_proof_byte_fails() {
        let mut proof = b(FIX_PROOF);
        let n = proof.len() - 1;
        proof[n] ^= 0x01;
        assert!(groth16::verify(&b(FIX_VKEY), &b(FIX_PUB), &proof).is_err());
    }

    #[test]
    fn wrong_selector_fails() {
        let mut proof = b(FIX_PROOF);
        proof[0] ^= 0x01;
        assert_eq!(
            groth16::verify(&b(FIX_VKEY), &b(FIX_PUB), &proof),
            Err("wrong verifier selector".to_string())
        );
    }

    #[test]
    fn wrong_public_values_fails() {
        let mut pub_v = b(FIX_PUB);
        pub_v[0] ^= 0x01;
        assert!(groth16::verify(&b(FIX_VKEY), &pub_v, &b(FIX_PROOF)).is_err());
    }

    #[test]
    fn wrong_program_vkey_fails() {
        let mut vkey = b(FIX_VKEY);
        vkey[0] ^= 0x01;
        assert!(groth16::verify(&vkey, &b(FIX_PUB), &b(FIX_PROOF)).is_err());
    }

    #[test]
    fn groth16_opcode_isright_to_bool() {
        // Opcode boundary: real proof -> true.
        let ok = groth16_verify(&[
            Value::Str(FIX_VKEY.into()),
            Value::Str(FIX_PUB.into()),
            Value::Str(FIX_PROOF.into()),
        ])
        .unwrap();
        assert!(matches!(ok, Value::Bool(true)));
        // Wrong-width vkey -> Err (a JsonLogicException), NOT false.
        let bad_vkey = format!("0x{}", "00".repeat(31)); // 31 bytes
        let err = groth16_verify(&[
            Value::Str(bad_vkey),
            Value::Str(FIX_PUB.into()),
            Value::Str(FIX_PROOF.into()),
        ]);
        assert!(err.is_err(), "wrong-width vkey must be an opcode error");
    }
}

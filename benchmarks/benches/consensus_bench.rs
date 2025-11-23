use criterion::{black_box, criterion_group, criterion_main, Criterion};
use consensus::{EnhancedConsensus, ValidatorInfo};
use common::types::Header;
use common::traits::Consensus;
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use rand::RngCore;

fn benchmark_signature_verification(c: &mut Criterion) {
    // Setup validator
    let mut csprng = OsRng;
    let mut secret_bytes = [0u8; 32];
    csprng.fill_bytes(&mut secret_bytes);
    let signing_key = SigningKey::from_bytes(&secret_bytes);
    let verifying_key = signing_key.verifying_key();
    let public_key = verifying_key.to_bytes().to_vec();
    
    let validators = vec![ValidatorInfo {
        public_key,
        stake: 100,
        slashed: false,
    }];
    
    let consensus = EnhancedConsensus::new(validators);
    
    // Create signed header
    let mut header = Header {
        parent_hash: [0u8; 32],
        state_root: [0; 32],
        extrinsics_root: [0; 32],
        slot: 1,
        epoch: 0,
        validator_set_id: 0,
        signature: vec![],
        gas_used: 0,
        base_fee: 0,
    };
    
    let message = header.hash();
    let signature = signing_key.sign(&message);
    header.signature = signature.to_vec();

    c.bench_function("verify_header_signature", |b| {
        b.iter(|| {
            consensus.verify_header(black_box(&header)).unwrap();
        })
    });
}

criterion_group!(benches, benchmark_signature_verification);
criterion_main!(benches);

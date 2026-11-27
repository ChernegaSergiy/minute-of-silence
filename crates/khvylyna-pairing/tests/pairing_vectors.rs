use khvylyna_pairing::{
    pake::{SenderExchange, receiver_response},
    protocol,
};
use rand_core::{CryptoRng, RngCore};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Vectors {
    version: u8,
    suite: u8,
    code: String,
    max_payload: u16,
    sender_seed: u8,
    receiver_seed: u8,
    confirm_seed: u8,
    data_seed: u8,
    ack_seed: u8,
    plaintext: String,
    pake_init: String,
    pake_response: String,
    confirm: String,
    data: String,
    ack: String,
}

struct FixedRng {
    seed: u8,
    cursor: u8,
}

impl FixedRng {
    fn new(seed: u8) -> Self {
        Self { seed, cursor: 0 }
    }

    fn next_byte(&mut self) -> u8 {
        let byte = self.seed.wrapping_add(self.cursor);
        self.cursor = self.cursor.wrapping_add(1);
        byte
    }
}

impl RngCore for FixedRng {
    fn next_u32(&mut self) -> u32 {
        u32::from_le_bytes([
            self.next_byte(),
            self.next_byte(),
            self.next_byte(),
            self.next_byte(),
        ])
    }

    fn next_u64(&mut self) -> u64 {
        u64::from_le_bytes([
            self.next_byte(),
            self.next_byte(),
            self.next_byte(),
            self.next_byte(),
            self.next_byte(),
            self.next_byte(),
            self.next_byte(),
            self.next_byte(),
        ])
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        for byte in dest {
            *byte = self.next_byte();
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}

impl CryptoRng for FixedRng {}

#[test]
fn desktop_matches_shared_pairing_vectors() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/pairing_vectors.json"
    );
    let vectors: Vectors = serde_json::from_str(&std::fs::read_to_string(path).expect("vectors"))
        .expect("valid vectors");
    assert_eq!(vectors.version, protocol::VERSION);
    assert_eq!(vectors.suite, protocol::SUITE_CPACE_RISTRETTO_AES256_GCM);

    let (sender, init) = SenderExchange::start(
        &vectors.code,
        vectors.max_payload,
        &mut FixedRng::new(vectors.sender_seed),
    )
    .expect("sender");
    assert_eq!(hex::encode(init), vectors.pake_init);

    let (attempt, response) = receiver_response(
        &vectors.code,
        &hex::decode(&vectors.pake_init).expect("init hex"),
        vectors.max_payload,
        &mut FixedRng::new(vectors.receiver_seed),
    )
    .expect("receiver");
    assert_eq!(hex::encode(&response), vectors.pake_response);

    let sender_secret = sender
        .finish(&vectors.code, &response)
        .expect("sender finish");
    let confirm = sender_secret
        .seal_confirm(&mut FixedRng::new(vectors.confirm_seed))
        .expect("confirm");
    assert_eq!(hex::encode(&confirm), vectors.confirm);
    let confirmed = attempt.verify_confirm(&confirm).expect("verify confirm");
    let receiver_secret = confirmed.into_secret();

    let plaintext = hex::decode(&vectors.plaintext).expect("plaintext hex");
    let data = sender_secret
        .seal_data(&plaintext, &mut FixedRng::new(vectors.data_seed))
        .expect("data");
    assert_eq!(hex::encode(&data), vectors.data);
    assert_eq!(
        receiver_secret
            .open_data(&data)
            .expect("open data")
            .as_slice(),
        plaintext
    );

    let ack = receiver_secret
        .seal_ack(true, &mut FixedRng::new(vectors.ack_seed))
        .expect("ack");
    assert_eq!(hex::encode(&ack), vectors.ack);
    assert!(sender_secret.open_ack(&ack).expect("open ack"));
}

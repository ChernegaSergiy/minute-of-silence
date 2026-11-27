use khvylyna_pairing::{
    pake::{SenderExchange, receiver_response},
    protocol,
};
use rand_core::{CryptoRng, RngCore};

struct FixedRng {
    seed: u8,
    cursor: u8,
}

impl FixedRng {
    fn new(seed: u8) -> Self {
        Self { seed, cursor: 0 }
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

impl FixedRng {
    fn next_byte(&mut self) -> u8 {
        let byte = self.seed.wrapping_add(self.cursor);
        self.cursor = self.cursor.wrapping_add(1);
        byte
    }
}

impl CryptoRng for FixedRng {}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn main() {
    let code = "123456";
    let max_payload = 2048u16;
    let plaintext = b"private settings";

    let (sender, init) =
        SenderExchange::start(code, max_payload, &mut FixedRng::new(1)).expect("sender hello");
    let (attempt, response) = receiver_response(code, &init, max_payload, &mut FixedRng::new(2))
        .expect("receiver response");
    let sender_secret = sender.finish(code, &response).expect("sender finish");
    let confirm = sender_secret
        .seal_confirm(&mut FixedRng::new(3))
        .expect("confirm");
    attempt.verify_confirm(&confirm).expect("verify confirm");
    let receiver_secret = attempt.into_secret();

    let data = sender_secret
        .seal_data(plaintext, &mut FixedRng::new(4))
        .expect("seal data");
    let opened = receiver_secret.open_data(&data).expect("open data");
    assert_eq!(opened.as_slice(), plaintext);
    let ack = receiver_secret
        .seal_ack(true, &mut FixedRng::new(5))
        .expect("seal ack");
    assert!(sender_secret.open_ack(&ack).expect("open ack"));

    let vector = serde_json::json!({
        "version": protocol::VERSION,
        "suite": protocol::SUITE_CPACE_RISTRETTO_AES256_GCM,
        "code": code,
        "maxPayload": max_payload,
        "senderSeed": 1,
        "receiverSeed": 2,
        "confirmSeed": 3,
        "dataSeed": 4,
        "ackSeed": 5,
        "plaintext": hex(plaintext),
        "pakeInit": hex(&init),
        "pakeResponse": hex(&response),
        "confirm": hex(&confirm),
        "data": hex(&data),
        "ack": hex(&ack),
    });
    println!("{}", serde_json::to_string_pretty(&vector).expect("json"));
}

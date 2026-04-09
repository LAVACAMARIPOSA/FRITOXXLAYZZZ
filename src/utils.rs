use solana_sdk::signature::{read_keypair_file, Keypair};
use std::env;

pub fn load_keypair(path: &str) -> Keypair {
    if let Ok(raw_keypair) = env::var("SOLANA_KEYPAIR_JSON") {
        let bytes: Vec<u8> = serde_json::from_str(&raw_keypair)
            .expect("❌ SOLANA_KEYPAIR_JSON no es un array JSON valido de 64 bytes");
        return Keypair::from_bytes(&bytes)
            .expect("❌ SOLANA_KEYPAIR_JSON no contiene un keypair valido");
    }

    read_keypair_file(path).expect(
        "❌ Crea keypair.json con: solana-keygen new -o keypair.json --no-passphrase o define SOLANA_KEYPAIR_JSON en el entorno",
    )
}

pub fn log_success(msg: &str) {
    println!("✅ {}", msg);
}

pub fn log_error(msg: &str) {
    eprintln!("❌ {}", msg);
}

pub fn log_info(msg: &str) {
    println!("🔍 {}", msg);
}

pub fn log_warning(msg: &str) {
    println!("⚠️ {}", msg);
}
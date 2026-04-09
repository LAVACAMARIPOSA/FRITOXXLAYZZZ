use solana_sdk::signature::{read_keypair_file, Keypair};

pub fn load_keypair(path: &str) -> Keypair {
    read_keypair_file(path)
        .expect("❌ Crea keypair.json con: solana-keygen new -o keypair.json --no-passphrase")
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
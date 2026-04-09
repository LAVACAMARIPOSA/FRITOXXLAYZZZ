use solana_sdk::signature::Keypair;
use std::env;

pub fn load_keypair() -> Keypair {
    // Para Render (variable de entorno)
    if let Ok(keypair_json) = env::var("SOLANA_KEYPAIR_JSON") {
        let bytes: Vec<u8> = serde_json::from_str(&keypair_json)
            .expect("❌ SOLANA_KEYPAIR_JSON debe ser un JSON array valido de 64 bytes");
        if bytes.len() != 64 {
            panic!("❌ Keypair invalido: debe tener exactamente 64 bytes");
        }
        Keypair::from_bytes(&bytes).expect("❌ Error creando Keypair desde JSON")
    }
    // Fallback local
    else {
        let path = "keypair.json";
        if std::path::Path::new(path).exists() {
            let bytes: Vec<u8> = std::fs::read(path).expect("Error leyendo keypair.json");
            let bytes: Vec<u8> =
                serde_json::from_slice(&bytes).expect("Error parseando keypair.json");
            Keypair::from_bytes(&bytes).expect("Error creando Keypair")
        } else {
            panic!("❌ Define SOLANA_KEYPAIR_JSON en Render o crea keypair.json localmente");
        }
    }
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
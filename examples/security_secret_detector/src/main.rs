//! Entry point for the `security_secret_detector` example binary.

#![forbid(unsafe_code)]

use security_secret_detector::{SecretDetector, redact_text};

fn main() {
    tracing_subscriber::fmt::init();

    let sample_text = "\
# Application configuration
API_KEY = \"sec_1234567890123456\"
DATABASE_URL = \"postgres://user:sk_live_abcdef1234567890@db.internal:5432/app\"
# Note: token_count = 100 is not a secret
";

    println!("--- Input Text ---");
    println!("{sample_text}");

    let detector = SecretDetector::default();
    let matches = detector.scan(sample_text);

    println!("--- Detected Findings (Safe Masked Output) ---");
    for m in &matches {
        println!("{m}");
    }

    println!("\n--- Redacted Text ---");
    let redacted = redact_text(sample_text, &matches);
    println!("{redacted}");
}

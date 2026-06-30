use std::process::Command;

fn main() {
    let status = Command::new("bash")
        .arg("scripts/roast-scorer.sh")
        .status()
        .expect("failed to execute roast-scorer.sh");

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
}

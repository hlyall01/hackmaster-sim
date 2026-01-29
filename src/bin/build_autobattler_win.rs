use std::process::{Command, ExitStatus};

fn run_cargo(cargo: &str, args: &[&str]) -> ExitStatus {
    Command::new(cargo)
        .args(args)
        .status()
        .unwrap_or_else(|err| panic!("Failed to run {cargo}: {err}"))
}

fn exit_code(status: ExitStatus) -> i32 {
    status.code().unwrap_or(1)
}

fn main() {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = run_cargo(
        &cargo,
        &[
            "build",
            "--release",
            "--target",
            "x86_64-pc-windows-gnu",
            "--features",
            "bevy",
            "--bin",
            "autobattler",
        ],
    );
    if !status.success() {
        std::process::exit(exit_code(status));
    }

    let status = run_cargo(
        &cargo,
        &[
            "build",
            "--release",
            "--target",
            "x86_64-pc-windows-gnu",
            "--bins",
        ],
    );
    std::process::exit(exit_code(status));
}

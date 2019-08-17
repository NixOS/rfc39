use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;

pub fn nix_instantiate_file_to_struct<T>(
    logger: slog::Logger,
    file: &Path,
) -> Result<T, serde_json::error::Error>
where
    T: serde::de::DeserializeOwned,
{
    let output = Command::new("nix-instantiate")
        .args(&["--eval", "--strict", "--json"])
        .arg(file)
        .output()
        .expect("Failed to start nix-instantiate!");

    if !output.stderr.is_empty() {
        warn!(logger, "Stderr from nix-instantiate";
              "stderr" => String::from_utf8_lossy(&output.stderr).to_string()
        );
        // "stderr" => stderr);
    }

    if !output.status.success() {
        panic!(
            "Nix failed! {:#?}",
            String::from_utf8_lossy(&output.stderr).to_string()
        );
    }

    serde_json::from_slice(&output.stdout)
}

pub fn nix_instantiate_expr_args_to_struct<T>(
    logger: slog::Logger,
    expr: &str,
    args: Vec<(&str, &OsStr)>,
) -> Result<T, serde_json::error::Error>
where
    T: serde::de::DeserializeOwned,
{
    let mut cmd = Command::new("nix-instantiate");
    cmd.args(&["--eval", "--strict", "--json", "--expr"]);
    cmd.arg(expr);

    for (arg, val) in args.into_iter() {
        cmd.arg("--arg");
        cmd.arg(arg);
        cmd.arg(val);
    }

    let output = cmd.output().expect("F;ailed to start nix-instantiate!");

    if !output.stderr.is_empty() {
        warn!(logger, "Stderr from nix-instantiate";
              "stderr" => String::from_utf8_lossy(&output.stderr).to_string()
        );
        // "stderr" => stderr);
    }

    if !output.status.success() {
        panic!(
            "Nix failed! {:#?}",
            String::from_utf8_lossy(&output.stderr).to_string()
        );
    }

    serde_json::from_slice(&output.stdout)
}

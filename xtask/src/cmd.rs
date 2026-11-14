use std::process::Command;

pub fn run(command: &mut Command, what: &str) -> Result<(), String> {
    let status = command
        .status()
        .map_err(|err| format!("{what}: could not start: {err}"))?;

    if !status.success() {
        return Err(format!("{what} exited with {status}"));
    }

    Ok(())
}

pub fn output(command: &mut Command, what: &str) -> Result<String, String> {
    let output = command
        .output()
        .map_err(|err| format!("{what}: could not start: {err}"))?;

    if !output.status.success() {
        return Err(format!("{what} exited with {}", output.status));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

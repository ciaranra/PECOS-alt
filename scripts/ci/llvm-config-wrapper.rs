use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, ExitCode};

fn llvm_lib_exists(current_exe: &Path, lib_name: &str) -> bool {
    let Some(bin_dir) = current_exe.parent() else {
        return false;
    };
    let Some(prefix_dir) = bin_dir.parent() else {
        return false;
    };
    prefix_dir.join("lib").join(lib_name).exists()
}

fn filter_system_libs(current_exe: &Path, stdout: &[u8]) -> Vec<u8> {
    let output = String::from_utf8_lossy(stdout);
    let mut filtered = output
        .split_whitespace()
        .filter(|token| {
            !token.eq_ignore_ascii_case("libxml2s.lib") || llvm_lib_exists(current_exe, token)
        })
        .collect::<Vec<_>>()
        .join(" ");

    if output.ends_with('\n') {
        filtered.push('\n');
    }
    filtered.into_bytes()
}

fn main() -> ExitCode {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    let current_exe = match env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            let _ = writeln!(
                io::stderr(),
                "failed to locate llvm-config wrapper: {error}"
            );
            return ExitCode::FAILURE;
        }
    };
    let real_config = current_exe.with_file_name("llvm-config.real.exe");
    let output = match Command::new(&real_config).args(&args).output() {
        Ok(output) => output,
        Err(error) => {
            let _ = writeln!(
                io::stderr(),
                "failed to run {}: {error}",
                real_config.display()
            );
            return ExitCode::FAILURE;
        }
    };

    let _ = io::stderr().write_all(&output.stderr);
    let is_system_libs = args.iter().any(|arg| arg == "--system-libs");
    let stdout = if is_system_libs && output.status.success() {
        filter_system_libs(&current_exe, &output.stdout)
    } else {
        output.stdout
    };
    let _ = io::stdout().write_all(&stdout);

    match output.status.code() {
        Some(code) if (0..=255).contains(&code) => ExitCode::from(code as u8),
        Some(_) | None => ExitCode::FAILURE,
    }
}

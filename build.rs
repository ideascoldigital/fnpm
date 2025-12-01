use std::process::Command;

fn main() {
    // Get git tag version
    let mut version_set = false;

    if let Ok(output) = Command::new("git")
        .args(["describe", "--tags", "--exact-match"])
        .output()
    {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("cargo:rustc-env=FNPM_VERSION={version}");
            version_set = true;
        }
    }

    if !version_set {
        // Fallback to git commit hash if no exact tag
        if let Ok(output) = Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
        {
            if output.status.success() {
                let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
                println!("cargo:rustc-env=FNPM_VERSION=dev-{commit}");
                version_set = true;
            }
        }
    }

    // Final fallback if git is not available
    if !version_set {
        println!("cargo:rustc-env=FNPM_VERSION=0.1.1-dev");
    }

    // Get git commit hash
    if let Ok(output) = Command::new("git").args(["rev-parse", "HEAD"]).output() {
        if output.status.success() {
            let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("cargo:rustc-env=FNPM_COMMIT={commit}");
        } else {
            println!("cargo:rustc-env=FNPM_COMMIT=unknown");
        }
    } else {
        println!("cargo:rustc-env=FNPM_COMMIT=unknown");
    }

    // Get build date
    let build_date = chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S UTC")
        .to_string();
    println!("cargo:rustc-env=FNPM_BUILD_DATE={build_date}");

    // Re-run if git changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/");
}

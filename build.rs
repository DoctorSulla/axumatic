use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=frontend/src");
    println!("cargo:rerun-if-changed=frontend/package.json");

    let pnpm_install = Command::new("pnpm")
        .args(["install"])
        .current_dir("./frontend")
        .status()
        .expect("Failed to run pnpm install");

    if !pnpm_install.success() {
        panic!(
            "pnpm install failed, please check you have it installed and available on your path"
        );
    }

    let pnpm_build = Command::new("pnpm")
        .args(["run", "build"])
        .current_dir("./frontend")
        .status()
        .expect("Failed to run pnpm run build");

    if !pnpm_build.success() {
        panic!(
            "pnpm run build failed, please check you have it installed and available on your path"
        );
    }
}

// Update build.rs to point to the new main.slint location
fn main() {
    println!("cargo:rerun-if-changed=src/ui/sl/main.slint");
    println!("cargo:rerun-if-changed=src/ui/assets/fonts");
    slint_build::compile("src/ui/sl/main.slint").unwrap();
}

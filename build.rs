fn main() {
    println!("cargo:rerun-if-changed=src/ui/sl");
    println!("cargo:rerun-if-changed=src/ui/assets/fonts");
    slint_build::compile("src/ui/sl/app.slint").unwrap();
}

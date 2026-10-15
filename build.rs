fn main() {
    println!("cargo:rerun-if-changed=src/ui/sl/app.slint");
    println!("cargo:rerun-if-changed=src/ui/sl/main.slint");
    println!("cargo:rerun-if-changed=src/ui/sl/chat.slint");
    println!("cargo:rerun-if-changed=src/ui/sl/shared.slint");
    println!("cargo:rerun-if-changed=src/ui/sl/theme.slint");
    println!("cargo:rerun-if-changed=src/ui/assets/fonts");
    slint_build::compile("src/ui/sl/app.slint").unwrap();
}

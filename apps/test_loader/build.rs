fn main() {
    println!("cargo:rerun-if-changed=c/b.c");
    println!("cargo:rerun-if-changed=c/a.c");
}

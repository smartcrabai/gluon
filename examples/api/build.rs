fn main() {
    if let Err(error) = gluon_build::run() {
        eprintln!("gluon build failed: {error}");
        std::process::exit(1);
    }
}

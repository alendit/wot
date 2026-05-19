fn main() {
    if let Err(error) = wot::cli::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

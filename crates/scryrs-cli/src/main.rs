fn main() {
    let exit_code = scryrs_cli::run(std::env::args().skip(1));
    std::process::exit(exit_code);
}

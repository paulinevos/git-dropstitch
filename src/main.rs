use std::process::exit;

use clap::Parser;

use gds_lib::Cli;
use gds_lib::dropstitch::Dropstitch;

fn main() {
    let cli = Cli::parse();

    match Dropstitch::run(cli) {
        Ok(_) => (),
        Err(e) => {
            e.display_for_user();
            exit(e.into_exit_code());
        }
    };
}

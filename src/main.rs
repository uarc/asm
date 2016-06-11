extern crate clap;
use clap::{App, Arg};

fn main() {
    let matches = App::new("uarc-asm")
        .version("0.1.0")
        .author("Geordon Worley <vadixidav@gmail.com>")
        .about("A configurable, generic assembler")
        .arg(Arg::with_name("format")
            .long("format")
            .short("f")
            .takes_value(true)
            .possible_values(&["little-endian", "big-endian", "hex-list"])
            .help("The format of the output files"))
        .arg(Arg::with_name("config")
            .long("config")
            .short("c")
            .help("Input JSON configuration file")
            .required(true))
        .arg(Arg::with_name("outputs")
            .long("outputs")
            .short("o")
            .help("List of the output file names"))
        .arg(Arg::with_name("inputs")
            .index(1)
            .help("List of the input assembly files in the order they are parsed"))
        .get_matches();
}

use lofi_core::Micros;
use std::env;
use std::path::PathBuf;

use lofi_sim::{DEFAULT_GROUP_JOIN_US, DEFAULT_SYNC_START_US};

#[derive(Debug)]
pub struct Args {
    pub nodes: usize,
    pub duration_ms: Micros,
    pub settle_ms: Micros,
    pub sync_start_ms: Micros,
    pub group_join_ms: Micros,
    pub seed: u64,
    pub wav_path: PathBuf,
}

impl Args {
    pub fn parse() -> Self {
        let mut args = env::args().skip(1);
        let mut out = Self {
            nodes: 8,
            duration_ms: 18_000,
            settle_ms: 0,
            sync_start_ms: DEFAULT_SYNC_START_US / 1_000,
            group_join_ms: DEFAULT_GROUP_JOIN_US / 1_000,
            seed: 0x10f1,
            wav_path: PathBuf::from("target/lofi-two-clusters-merge.wav"),
        };

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--nodes" => out.nodes = parse_next(&mut args, "--nodes"),
                "--duration-ms" => out.duration_ms = parse_next(&mut args, "--duration-ms"),
                "--settle-ms" => out.settle_ms = parse_next(&mut args, "--settle-ms"),
                "--sync-start-ms" => out.sync_start_ms = parse_next(&mut args, "--sync-start-ms"),
                "--group-join-ms" => out.group_join_ms = parse_next(&mut args, "--group-join-ms"),
                "--seed" => out.seed = parse_next(&mut args, "--seed"),
                "--wav" => out.wav_path = PathBuf::from(args.next().expect("--wav needs a path")),
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                _ => panic!("unknown argument {arg}; try --help"),
            }
        }

        out.nodes = out.nodes.max(1);
        out
    }
}

fn parse_next<T: std::str::FromStr>(args: &mut impl Iterator<Item = String>, name: &str) -> T {
    args.next()
        .unwrap_or_else(|| panic!("{name} needs a value"))
        .parse()
        .unwrap_or_else(|_| panic!("invalid value for {name}"))
}

fn print_help() {
    println!(
        "lofi-sim [--nodes N] [--settle-ms MS] [--sync-start-ms MS] [--group-join-ms MS] [--duration-ms MS] [--seed N] [--wav PATH]"
    );
}

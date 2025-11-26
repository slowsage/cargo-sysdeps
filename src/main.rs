use anyhow::Result;
use clap::{Parser, Subcommand};

mod distro;
mod index;
mod scanner;

#[derive(Parser)]
#[command(name = "cargo", bin_name = "cargo")]
enum Cli {
    Sysdeps(Args),
}

#[derive(clap::Args)]
struct Args {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Generate {
        #[arg(short, long)]
        distro: Option<String>,
        #[arg(long)]
        stream: bool,
    },
    Install {
        #[arg(short, long)]
        input: Option<String>,
        #[arg(short, long)]
        distro: Option<String>,
        #[arg(long)]
        arch: Option<String>,
    },
    CrossSetup {
        #[arg(long)]
        arch: String,
        #[arg(short, long)]
        distro: Option<String>,
    },
}

fn main() -> Result<()> {
    match Cli::parse() {
        Cli::Sysdeps(a) => match a.cmd {
            Cmd::Generate { distro, stream } => {
                let d = distro::resolve(distro)?;
                let deps = scanner::scan()?;
                for p in index::resolve(&deps, &d, stream)? {
                    println!("{}", p);
                }
            }
            Cmd::Install {
                input,
                distro,
                arch,
            } => {
                let d = distro::resolve(distro)?;
                distro::install(input, &d, arch)?;
            }
            Cmd::CrossSetup { arch, distro } => {
                let d = distro::resolve(distro)?;
                distro::cross_setup(&d, &arch)?;
            }
        },
    }
    Ok(())
}

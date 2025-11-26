use anyhow::{Context, Result, anyhow};
use std::{fs, io::{self, Read}, process::Command};

#[derive(Debug, Clone)]
pub struct Distro { pub name: String, pub version: String }

pub fn resolve(input: Option<String>) -> Result<Distro> {
    if let Some(s) = input {
        let (n, v) = s.split_once('-').unwrap_or(("arch", ""));
        let v = if v.chars().next().is_some_and(|c| c.is_ascii_digit()) { fetch_codename(n, v)? } else { v.into() };
        Ok(Distro { name: n.into(), version: v })
    } else {
        let os = fs::read_to_string("/etc/os-release").context("No /etc/os-release")?;
        let (mut n, mut v, mut c) = (String::new(), String::new(), String::new());
        for l in os.lines() {
            if let Some((k, val)) = l.split_once('=') {
                match k {
                    "ID" => n = val.trim_matches('"').into(),
                    "VERSION_ID" => v = val.trim_matches('"').into(),
                    "VERSION_CODENAME" => c = val.trim_matches('"').into(),
                    _ => {}
                }
            }
        }
        Ok(Distro { name: n, version: if !c.is_empty() { c } else { v } })
    }
}

fn fetch_codename(distro: &str, ver: &str) -> Result<String> {
    let url = match distro {
        "debian" => "https://salsa.debian.org/debian/distro-info-data/-/raw/main/debian.csv",
        "ubuntu" => "https://salsa.debian.org/debian/distro-info-data/-/raw/main/ubuntu.csv",
        _ => return Ok(ver.into()),
    };
    for line in ureq::get(url).call()?.into_string()?.lines() {
        let cols: Vec<&str> = line.split(',').collect();
        if cols.first().is_some_and(|c| *c == ver || c.starts_with(&format!("{} ", ver))) {
            return Ok(cols.get(1).context("No codename")?.to_string());
        }
    }
    Ok(ver.into())
}

pub fn install(input: Option<String>, d: &Distro, arch: Option<String>) -> Result<()> {
    let mut buf = String::new();
    if let Some(p) = input { buf = fs::read_to_string(p)?; } else { io::stdin().read_to_string(&mut buf)?; }
    let pkgs: Vec<&str> = buf.lines().filter(|l| !l.is_empty()).collect();
    if pkgs.is_empty() { return Ok(()); }

    let (cmd, base_args) = match d.name.as_str() {
        "debian" | "ubuntu" => ("apt-get", vec!["install", "-y"]),
        "arch" => ("pacman", vec!["-S", "--needed", "--noconfirm"]),
        _ => return Err(anyhow!("Unsupported: {}", d.name)),
    };
    
    let pkg_args = pkgs.iter().map(|p| {
        if let Some(a) = &arch { format!("{}:{}", p, a) } else { p.to_string() }
    });

    Command::new(cmd).args(base_args).args(pkg_args).status()?;
    Ok(())
}

pub fn cross_setup(d: &Distro, arch: &str) -> Result<()> {
    if matches!(d.name.as_str(), "debian" | "ubuntu") {
        Command::new("dpkg").args(["--add-architecture", arch]).status()?;
        Command::new("apt-get").arg("update").status()?;
    }
    Ok(())
}

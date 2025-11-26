use crate::distro::Distro;
use anyhow::{Context, Result};
use directories::ProjectDirs;
use flate2::read::GzDecoder;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;
use tar::Archive;

pub fn resolve(deps: &[String], d: &Distro, stream: bool) -> Result<HashSet<String>> {
    let cache = ProjectDirs::from("", "", "cargo-sysdeps").context("No cache")?.cache_dir().to_path_buf();
    fs::create_dir_all(&cache)?;
    let idx = cache.join(format!("{}-{}-pc.index", d.name, d.version));
    let mut map: HashMap<String, String> = HashMap::new();

    if idx.exists() {
        for l in io::BufReader::new(File::open(&idx)?).lines() {
            if let Some((k, v)) = l?.split_once(' ') { map.insert(k.into(), v.into()); }
        }
    } else {
        let urls = match d.name.as_str() {
            "debian" => vec![format!("http://deb.debian.org/debian/dists/{}/main/Contents-amd64.gz", d.version)],
            "ubuntu" => vec![
                format!("http://archive.ubuntu.com/ubuntu/dists/{}/main/Contents-amd64.gz", d.version),
                format!("http://archive.ubuntu.com/ubuntu/dists/{}/universe/Contents-amd64.gz", d.version),
            ],
            "arch" => vec![
                "https://mirrors.kernel.org/archlinux/core/os/x86_64/core.files.tar.gz".into(),
                "https://mirrors.kernel.org/archlinux/extra/os/x86_64/extra.files.tar.gz".into(),
            ],
            _ => vec![],
        };

        for url in urls {
            let name = url.split('/').next_back().unwrap();
            let raw = cache.join(name);
            let reader: Box<dyn Read> = if !stream && raw.exists() {
                Box::new(File::open(&raw)?)
            } else {
                eprintln!("DL {}...", url);
                let mut resp = ureq::get(&url).call()?.into_reader();
                if !stream {
                    let mut f = File::create(&raw)?;
                    io::copy(&mut resp, &mut f)?;
                    Box::new(File::open(&raw)?)
                } else { Box::new(resp) }
            };

            if name.ends_with(".tar.gz") {
                for e in Archive::new(GzDecoder::new(reader)).entries()? {
                    let e = e?;
                    let p = e.path()?;
                    if p.extension().is_some_and(|x| x == "pc")
                        && let (Some(pkg), Some(stem)) = (p.components().next(), p.file_stem()) {
                            map.insert(stem.to_string_lossy().into(), pkg.as_os_str().to_string_lossy().into());
                        }
                }
            } else {
                for l in BufReader::new(GzDecoder::new(reader)).lines() {
                    let l = l?;
                    if l.contains(".pc") {
                        let p: Vec<&str> = l.split_whitespace().collect();
                        if p.len() >= 2 && p[0].contains("/pkgconfig/") && p[0].ends_with(".pc") {
                             let pkg = p.last().unwrap().split(',').next().unwrap();
                             let pkg_name = pkg.split('/').next_back().unwrap_or(pkg);
                             if let Some(stem) = Path::new(p[0]).file_stem() {
                                 map.insert(stem.to_string_lossy().into(), pkg_name.into());
                             }
                        }
                    }
                }
            }
        }
        let mut f = File::create(&idx)?;
        for (k, v) in &map { writeln!(f, "{} {}", k, v)?; }
    }

    Ok(deps.iter().filter_map(|d| {
        map.get(d).or_else(|| map.get(&d.replace('_', "-"))).cloned().or_else(|| {
            eprintln!("Missing: {}", d); None
        })
    }).collect())
}

use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Command;
use syn::visit::Visit;
use walkdir::WalkDir;

pub fn scan() -> Result<Vec<String>> {
    Command::new("cargo")
        .arg("fetch")
        .status()
        .context("cargo fetch failed")?;

    let meta = MetadataCommand::new().exec()?;

    let mut deps = HashSet::new();

    let mut node_features = HashMap::new();

    if let Some(resolve) = &meta.resolve {
        for node in &resolve.nodes {
            node_features.insert(node.id.clone(), node.features.clone());
        }
    }

    for pkg in meta.packages {
        // Only scan packages active in the dependency graph

        let active_features = match node_features.get(&pkg.id) {
            Some(f) => f,

            None => continue,
        };

        if let Some(t) = pkg.metadata.get("system-deps").and_then(|v| v.as_object()) {
            for (key, val) in t {
                let obj = val.as_object();

                let pkg_name = obj
                    .and_then(|o| o.get("name").and_then(|s| s.as_str()))
                    .unwrap_or(key)
                    .to_string();

                let feature = obj.and_then(|o| o.get("feature").and_then(|s| s.as_str()));

                let optional = obj
                    .and_then(|o| o.get("optional").and_then(|b| b.as_bool()))
                    .unwrap_or(false);

                let required = if let Some(feat) = feature {
                    active_features.iter().any(|f| f.as_str() == feat)
                } else if optional {
                    active_features.iter().any(|f| f.as_str() == key)
                } else {
                    true
                };

                if required {
                    deps.insert(pkg_name);
                }
            }
        }

        if let Some(dir) = pkg.manifest_path.parent().map(|p| p.as_std_path()) {
            let paths = [dir.join("build.rs")];

            for p in &paths {
                if p.exists() {
                    scan_file(p, &mut deps);
                }
            }

            let bdir = dir.join("build");

            if bdir.exists() {
                for e in WalkDir::new(bdir).into_iter().filter_map(|e| e.ok()) {
                    if e.path().extension().is_some_and(|x| x == "rs") {
                        scan_file(e.path(), &mut deps);
                    }
                }
            }
        }
    }

    Ok(deps.into_iter().collect())
}

fn scan_file(path: &Path, deps: &mut HashSet<String>) {
    if let Ok(s) = fs::read_to_string(path)
        && let Ok(ast) = syn::parse_file(&s) {
            let mut v = Visitor::default();
            v.visit_file(&ast);
            deps.extend(v.found);
        }
}

#[derive(Default)]
struct Visitor {
    found: HashSet<String>,
    vars: HashMap<String, String>,
}
impl<'a> Visit<'a> for Visitor {
    fn visit_local(&mut self, l: &'a syn::Local) {
        if let (Some(i), syn::Pat::Ident(p)) = (&l.init, &l.pat)
            && let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &*i.expr
            {
                self.vars.insert(p.ident.to_string(), s.value());
            }
        syn::visit::visit_local(self, l);
    }
    fn visit_expr_method_call(&mut self, m: &'a syn::ExprMethodCall) {
        if m.method == "probe"
            && let Some(arg) = m.args.first() {
                match arg {
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) => {
                        self.found.insert(s.value());
                    }
                    syn::Expr::Path(p) => {
                        if let Some(i) = p.path.get_ident()
                            && let Some(v) = self.vars.get(&i.to_string()) {
                                self.found.insert(v.clone());
                            }
                    }
                    _ => {}
                }
            }
        syn::visit::visit_expr_method_call(self, m);
    }
}

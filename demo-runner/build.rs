// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use anyhow::{Context as _, anyhow};
use aya_build::Toolchain;

fn main() -> anyhow::Result<()> {
    let cargo_metadata::Metadata { packages, .. } = cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .context("MetadataCommand::exec")?;
    let ebpf_package = packages
        .into_iter()
        .find(|cargo_metadata::Package { name, .. }| name.as_str() == "ebpf")
        .ok_or_else(|| anyhow!("ebpf package not found"))?;
    let cargo_metadata::Package {
        name,
        manifest_path,
        ..
    } = ebpf_package;
    let root_dir = manifest_path
        .parent()
        .ok_or_else(|| anyhow!("no parent for {manifest_path}"))?
        .as_str();
    let toolchain = ebpf_toolchain(root_dir)?;
    let ebpf_package = aya_build::Package {
        name: name.as_str(),
        root_dir,
        features: &["with-inline-assembler"],
        ..Default::default()
    };
    aya_build::build_ebpf([ebpf_package], Toolchain::Custom(&toolchain))
}

/// Reads the pinned nightly channel from rust-toolchain.toml in the ebpf crate.
/// aya_build::build_ebpf() invokes the nested cargo via `rustup run <toolchain>`,
/// which ignores rust-toolchain.toml, so the pin must be passed explicitly.
fn ebpf_toolchain(ebpf_root: &str) -> anyhow::Result<String> {
    let path = std::path::Path::new(ebpf_root).join("rust-toolchain.toml");
    println!("cargo:rerun-if-changed={}", path.display());
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", path.display()))?;
    content
        .lines()
        .find_map(|line| {
            let rest = line.strip_prefix("channel")?.trim_start().strip_prefix('=')?;
            Some(rest.trim().trim_matches('"').to_string())
        })
        .ok_or_else(|| anyhow!("no `channel` entry in {}", path.display()))
}

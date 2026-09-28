//! Probe system libspeexdsp (pkg-config). When present, link it and enable
//! `blightnet_has_aec` so `src/aec.rs` uses real Speex AEC. When absent, the
//! crate still builds with a passthrough stub — voice keeps working.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    // Declare custom cfg so rustc does not warn on #[cfg(blightnet_has_aec)].
    println!("cargo:rustc-check-cfg=cfg(blightnet_has_aec)");
    let ok = std::process::Command::new("pkg-config")
        .args(["--exists", "speexdsp"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        println!("cargo:rustc-cfg=blightnet_has_aec");
        println!("cargo:rustc-link-lib=speexdsp");
        if let Ok(out) = std::process::Command::new("pkg-config")
            .args(["--libs-only-L", "speexdsp"])
            .output()
        {
            let s = String::from_utf8_lossy(&out.stdout);
            for tok in s.split_whitespace() {
                if let Some(dir) = tok.strip_prefix("-L") {
                    println!("cargo:rustc-link-search=native={dir}");
                }
            }
        }
    } else {
        println!("cargo:warning=speexdsp not found via pkg-config; building without AEC (voice still works)");
    }
}

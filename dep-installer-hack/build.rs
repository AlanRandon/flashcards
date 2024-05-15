fn main() {
    // https://github.com/tectonic-typesetting/tectonic/blob/master/docs/src/howto/build-tectonic/external-dep-install.md
    let pkgs =
        "libfontconfig1-dev libgraphite2-dev libharfbuzz-dev libicu-dev libssl-dev zlib1g-dev poppler-utils";

    if std::env::var("HOSTNAME")
        .unwrap_or_default()
        .contains("shuttle")
    {
        if !std::process::Command::new("apt")
            .arg("install")
            .arg("-y")
            .args(pkgs.split(' '))
            .status()
            .expect("failed to run apt")
            .success()
        {
            panic!("failed to install dependencies");
        }

        std::process::Command::new("pdftocairo")
            .arg("-h")
            .status()
            .expect("pdftocairo to be runnable");
    } else {
        panic!("Run on shuttle")
    }

    println!("cargo:rustc-link-lib=graphite2")
}

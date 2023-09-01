use std::{
    io,
    process::{self, Command, ExitStatus},
};

trait StatusExt {
    fn exit_on_failure(&self) {}
}

impl StatusExt for io::Result<ExitStatus> {
    fn exit_on_failure(&self) {
        let Ok(status) = self else {
            process::exit(0);
        };

        let Some(code) = status.code() else {
            process::exit(0);
        };

        if !status.success() {
            process::exit(code);
        }
    }
}

fn main() {
    println!("cargo:rerun-if-changed=tailwind.config.ts");
    println!("cargo:rerun-if-changed=src");

    Command::new("./node_modules/.bin/tailwind")
        .args(["-i", "src/style.css", "-o", "dist/style.css", "--minify"])
        .status()
        .exit_on_failure();

    Command::new("./node_modules/.bin/esbuild")
        .args([
            "src/init.ts",
            "--outfile=dist/init.js",
            "--minify",
            "--bundle",
        ])
        .status()
        .exit_on_failure();
}

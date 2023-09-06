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
            println!("cargo:warning={:?}", self);
            process::exit(1);
        };

        let Some(code) = status.code() else {
            process::exit(1);
        };

        if !status.success() {
            process::exit(code);
        }
    }
}

fn main() {
    println!("cargo:rerun-if-changed=tailwind.config.ts");
    println!("cargo:rerun-if-changed=src");

    let bin = concat!(env!("CARGO_MANIFEST_DIR"), "/node_modules/.bin");
    let tailwind = format!("{bin}/tailwind -i src/style.css -o dist/style.css --minify");
    let esbuild = format!("{bin}/esbuild src/init.ts --outfile=dist/init.js --minify --bundle");

    if cfg!(target_os = "windows") {
        Command::new("cmd")
            .arg("/C")
            .arg(tailwind)
            .status()
            .exit_on_failure();

        Command::new("cmd")
            .arg("/C")
            .arg(esbuild)
            .status()
            .exit_on_failure();
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(tailwind)
            .status()
            .exit_on_failure();

        Command::new("sh")
            .arg("-c")
            .arg(esbuild)
            .status()
            .exit_on_failure();
    }
}

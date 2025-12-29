use base64::Engine;
use std::io::Write;
use std::process::Stdio;
use tectonic::config::PersistentConfig;
use tectonic::driver::ProcessingSessionBuilder;
use tectonic::status::NoopStatusBackend;
use tectonic::{ctry, driver, errmsg};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Tectonic failed to compile TeX")]
    Tectonic(#[from] tectonic::Error),
    #[error("IO failed")]
    Io(#[from] std::io::Error),
    #[error("pdftocairo failed")]
    PdfToCairo(std::process::ExitStatus),
}

#[test]
pub fn render_svg() {
    let source = r#"
\begin{tikzpicture}
\coordinate (origin) at (0:0);
\draw (origin) circle (1);
\coordinate (a) at (70:1);
\coordinate (b) at (150:1);
\coordinate (c) at (270:1);
\draw (a)--(b)--(c)--cycle;
\draw (c)--([turn]-90:1);
\draw (c)--([turn]90:1) coordinate (d);
\draw pic["$x$",draw,angle radius=8,angle eccentricity=1.7] {angle=d--c--a};
\draw pic["$x$",draw,angle radius=8,angle eccentricity=1.7] {angle=c--b--a};
\end{tikzpicture}
"#;
    render(source).unwrap();
}

pub fn render(source: &str) -> Result<String, Error> {
    let mut process = std::process::Command::new("pdftocairo")
        .arg("-")
        .args(["-", "-svg", "-f", "0", "-l", "0"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(Error::Io)?;

    let mut stdin = process.stdin.take().expect("child to have stdin");

    let data = tex_to_pdf(source)?;
    stdin.write_all(&data).map_err(Error::Io)?;
    stdin.flush().map_err(Error::Io)?;
    drop(stdin);

    let output = process.wait_with_output().map_err(Error::Io)?;
    if !output.status.success() {
        return Err(Error::PdfToCairo(output.status));
    }
    let data = output.stdout;

    let engine = base64::engine::GeneralPurpose::new(
        &base64::alphabet::STANDARD,
        base64::engine::GeneralPurposeConfig::new(),
    );
    let data = engine.encode(data);

    let mut escaped_source = String::new();
    pulldown_cmark_escape::escape_html(&mut escaped_source, source).unwrap();

    Ok(format!(
        r#"<img src="data:image/svg+xml;base64,{data}" alt="{escaped_source}" title="{escaped_source}" class="w-full h-full tex">"#
    ))
}

fn tex_to_pdf(source: &str) -> Result<Vec<u8>, Error> {
    let tex_input = format!(
        r"
\documentclass{{standalone}}
\usepackage{{chemfig}}
\usepackage{{mhchem}}
\usepackage{{circuitikz}}
\usepackage{{tikz}}
\usepackage{{adjustbox}}
\usetikzlibrary{{angles,quotes,calc,cd,decorations,decorations.markings,optics,intersections,patterns,shapes.misc}}
\usepackage{{xcolor}}
\definecolor{{base}}{{HTML}}{{1e293b}}
\begin{{document}}
\pagecolor{{base}}
\color{{white}}
\trimbox{{-.5cm -.5cm -.5cm -.5cm}}{{
{source}
}}
\end{{document}}
"
    );

    let config = ctry!(
        PersistentConfig::open(false);
        "failed to open config"
    );

    let mut status = NoopStatusBackend::default();

    let mut files = {
        let mut builder = ProcessingSessionBuilder::default();
        builder
            .bundle(ctry!(
                config.default_bundle(false, &mut status);
                "failed to load the default resource bundle"
            ))
            .primary_input_buffer(tex_input.as_bytes())
            .tex_input_name("input.tex")
            .format_name("latex")
            .format_cache_path(ctry!(
                config.format_cache_path();
                "failed to set up the format cache"
            ))
            .keep_logs(false)
            .keep_intermediates(false)
            .print_stdout(false)
            .output_format(driver::OutputFormat::Pdf)
            .do_not_write_output_files();

        let mut session = ctry!(
            builder.create(&mut status);
            "failed to initialize the LaTeX processing session"
        );
        ctry!(
            session.run(&mut status);
            "the LaTeX engine failed"
        );
        session.into_file_data()
    };

    let data = files
        .remove("input.pdf")
        .ok_or::<tectonic::Error>(errmsg!(
            "LaTeX didn't report failure, but no output was created (??)"
        ))?
        .data;

    Ok(data)
}

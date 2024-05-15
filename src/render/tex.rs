use base64::Engine;
use tectonic::config::PersistentConfig;
use tectonic::driver::ProcessingSessionBuilder;
use tectonic::status::NoopStatusBackend;
use tectonic::{ctry, driver, errmsg};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Tectonic failed to compile TeX")]
    Tectonic(#[from] tectonic::Error),
    #[error("IO failed")]
    IO(#[from] std::io::Error),
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
    let data = tex_to_pdf(source)?;
    let data = pdf_to_svg(&data)?;

    let engine = base64::engine::GeneralPurpose::new(
        &base64::alphabet::STANDARD,
        base64::engine::GeneralPurposeConfig::new(),
    );
    let data = engine.encode(data);

    let mut escaped_source = String::new();
    pulldown_cmark_escape::escape_html(&mut escaped_source, source).unwrap();

    Ok(format!(
        r#"<img src="data:image/svg+xml;base64,{data}" alt="{escaped_source}" title="{escaped_source}" class="tex">"#
    ))
}

fn pdf_to_svg(data: &[u8]) -> Result<Vec<u8>, Error> {
    std::fs::write("./dist/flashcard.pdf", data)?;
    std::process::Command::new("pdftocairo")
        .args([
            "./dist/flashcard.pdf",
            "./dist/flashcard.svg",
            "-svg",
            "-f",
            "0",
            "-l",
            "0",
        ])
        .status()
        .expect("pdftocairo");
    Ok(std::fs::read("./dist/flashcard.svg")?)
}

fn tex_to_pdf(source: &str) -> Result<Vec<u8>, Error> {
    let tex_input = format!(
        r#"
\documentclass{{standalone}}
\usepackage{{chemfig}}
\usepackage{{mhchem}}
\usepackage{{tikz}}
\usepackage{{adjustbox}}
\usetikzlibrary{{angles,quotes,calc}}
\usepackage{{xcolor}}
\definecolor{{base}}{{HTML}}{{1e293b}}
\begin{{document}}
\pagecolor{{base}}
\color{{white}}
\trimbox{{-.5cm -.5cm -.5cm -.5cm}}{{
{source}
}}
\end{{document}}
"#
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

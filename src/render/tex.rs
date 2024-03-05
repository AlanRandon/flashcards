use base64::Engine;
use itertools::Itertools;
use pathfinder_export::Export;
use tectonic::config::PersistentConfig;
use tectonic::driver::ProcessingSessionBuilder;
use tectonic::status::NoopStatusBackend;
use tectonic::{ctry, driver, errmsg};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Tectonic failed to compile TeX")]
    Tectonic(#[from] tectonic::Error),
    #[error("Failed to convert pdf to image")]
    Pdf(#[from] pdf::error::PdfError),
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
        r#"<img src="data:image/svg+xml;base64,{data}" alt="{escaped_source}" title="{escaped_source}">"#
    ))
}

fn pdf_to_svg(data: &[u8]) -> Result<Vec<u8>, Error> {
    std::env::set_var("STANDARD_FONTS", "dist/pdf-fonts");

    let file = pdf::file::FileOptions::uncached().load(data)?;
    let resolver = file.resolver();

    let mut cache = pdf_render::Cache::new();
    let mut backend = pdf_render::SceneBackend::new(&mut cache);

    file.pages()
        .map_ok(|page| pdf_render::render_page(&mut backend, &resolver, &page, Default::default()))
        .collect::<Result<Result<Vec<_>, _>, _>>()??;

    let scene = backend.finish();
    let mut data = Vec::<u8>::new();
    scene
        .export(&mut data, pathfinder_export::FileFormat::SVG)
        .unwrap();

    Ok(data)
}

fn tex_to_pdf(source: &str) -> Result<Vec<u8>, Error> {
    let tex_input = format!(
        r#"
\documentclass{{standalone}}
\usepackage{{chemfig}}
\usepackage{{mhchem}}
\begin{{document}}
{source}
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

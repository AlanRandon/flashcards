use crate::{Card, CardFormat};
use base64::Engine;
use pulldown_cmark as md;

mod katex_scanner {
    #[derive(Debug, Clone)]
    pub struct Scanner<'a> {
        input: &'a str,
        state: State,
        position: usize,
    }

    impl<'a> Scanner<'a> {
        pub fn new(input: &'a str) -> Self {
            Self {
                input,
                state: State::Text,
                position: 0,
            }
        }
    }

    const INLINE_DELIMETER: &str = "$";
    const BLOCK_DELIMETER: &str = "$$";

    impl<'a> Iterator for Scanner<'a> {
        type Item = Event<'a>;

        fn next(&mut self) -> Option<Self::Item> {
            let input = &self.input[self.position..];

            if input.is_empty() {
                return None;
            }

            match self.state {
                State::Text => {
                    let text = if let Some(position) = input.find(BLOCK_DELIMETER) {
                        let content = &input[..position];
                        self.position += position + BLOCK_DELIMETER.len();
                        self.state = State::Block;
                        content
                    } else if let Some(position) = input.find(INLINE_DELIMETER) {
                        let content = &input[..position];
                        self.position += position + INLINE_DELIMETER.len();
                        self.state = State::Inline;
                        content
                    } else {
                        self.position = self.input.len();
                        input
                    };
                    Some(Event::Text(text))
                }
                State::Block => {
                    let content = if let Some(position) = input.find(BLOCK_DELIMETER) {
                        let content = &input[..position];
                        self.position += position + BLOCK_DELIMETER.len();
                        self.state = State::Text;
                        Event::Block(content)
                    } else {
                        self.position = self.input.len();
                        Event::Text(input)
                    };
                    Some(content)
                }
                State::Inline => {
                    let content = if let Some(position) = input.find(INLINE_DELIMETER) {
                        let content = &input[..position];
                        self.position += position + INLINE_DELIMETER.len();
                        self.state = State::Text;
                        Event::Inline(content)
                    } else {
                        self.position = self.input.len();
                        Event::Text(input)
                    };
                    Some(content)
                }
            }
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub enum State {
        Text,
        Inline,
        Block,
    }

    #[derive(Debug)]
    pub enum Event<'a> {
        Text(&'a str),
        Inline(&'a str),
        Block(&'a str),
    }

    impl<'a> Event<'a> {
        pub fn render(&self) -> katex::Result<String> {
            use katex::Opts;

            match self {
                Self::Text(text) => Ok((*text).to_string()),
                Self::Block(content) => katex::render_with_opts(
                    content,
                    Opts::builder().display_mode(true).build().unwrap(),
                ),
                Self::Inline(content) => katex::render(content),
            }
        }

        pub fn as_html(&self) -> String {
            match self.render() {
                Ok(result) => result,
                Err(err) => {
                    eprintln!("Katex error ignored: {err}");
                    match self {
                        Self::Text(text) => (*text).to_string(),
                        Self::Block(content) => format!("$${content}$$"),
                        Self::Inline(content) => format!("${content}$"),
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct RenderedCard {
    pub card: Card,
    pub term: String,
    pub definition: String,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Malformated TeX")]
    TexError(#[from] tectonic::Error),
}

impl TryFrom<Card> for RenderedCard {
    type Error = Error;

    fn try_from(card: Card) -> Result<Self, Self::Error> {
        let (term, definition) = match card.format {
            CardFormat::Tex => (tex(&card.term)?, tex(&card.definition)?),
            CardFormat::Markdown => (markdown(&card.term), markdown(&card.definition)),
        };

        Ok(Self {
            card,
            term,
            definition,
        })
    }
}

pub fn markdown(text: &str) -> String {
    let scanner = katex_scanner::Scanner::new(text);
    let text = scanner.map(|event| event.as_html()).collect::<String>();
    let parser = md::Parser::new_ext(&text, md::Options::ENABLE_TABLES);

    let mut result = String::new();
    md::html::push_html(&mut result, parser);
    result
}

pub fn tex(text: &str) -> tectonic::Result<String> {
    use tectonic::{config, ctry, driver, errmsg, status};

    // tectonic::Spx2HtmlEngine::default().process_to_filesystem(, status, spx);

    let text = format!(
        r#"
\documentclass{{article}}
\usepackage{{chemfig}}
\begin{{document}}
{text}
\end{{document}}
"#
    );

    let mut status = status::NoopStatusBackend::default();

    let config = ctry!(
        config::PersistentConfig::open(false);
        "failed to open the default configuration file"
    );

    let only_cached = false;
    let bundle = ctry!(
        config.default_bundle(only_cached, &mut status);
        "failed to load the default resource bundle"
    );

    let format_cache_path = ctry!(
        config.format_cache_path();
        "failed to set up the format cache"
    );

    let mut files = {
        let mut builder = driver::ProcessingSessionBuilder::default();
        builder
            .bundle(bundle)
            .primary_input_buffer(text.as_bytes())
            .tex_input_name("input.tex")
            .format_name("latex")
            .format_cache_path(format_cache_path)
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

    // TODO: actual pdf to html

    let encoded =
        base64::engine::GeneralPurpose::new(&base64::alphabet::STANDARD, Default::default())
            .encode(data);

    Ok(format!(
        r#"<embed src="data:application/pdf;base64,{encoded}" width="500" height="500" 
 type="application/pdf">"#
    ))
}

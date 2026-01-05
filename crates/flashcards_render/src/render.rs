use crate::{Card, Format, Rendered, Source};
use base64::Engine;
use pulldown_cmark as md;

mod tex;

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

    #[test]
    fn katex_scanner_works() {
        let mut scanner = Scanner::new("$a$b$$c$$");
        assert_eq!(scanner.next().unwrap(), Event::Text(""));
        assert_eq!(scanner.next().unwrap(), Event::Inline("a"));
        assert_eq!(scanner.next().unwrap(), Event::Text("b"));
        assert_eq!(scanner.next().unwrap(), Event::Block("c"));
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
                    let text = match (input.find(INLINE_DELIMETER), input.find(BLOCK_DELIMETER)) {
                        (None, Some(_)) => {
                            unimplemented!("block delimeter should imply inline delimeter exists")
                        }
                        (Some(inline_start), Some(block_start)) if block_start <= inline_start => {
                            let content = &input[..block_start];
                            self.position += block_start + BLOCK_DELIMETER.len();
                            self.state = State::Block;
                            content
                        }
                        (Some(inline_start), _) => {
                            let content = &input[..inline_start];
                            self.position += inline_start + INLINE_DELIMETER.len();
                            self.state = State::Inline;
                            content
                        }
                        (None, None) => {
                            self.position = self.input.len();
                            input
                        }
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

    #[derive(Debug, PartialEq, Eq)]
    pub enum Event<'a> {
        Text(&'a str),
        Inline(&'a str),
        Block(&'a str),
    }

    impl Event<'_> {
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
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Malformatted TeX: {0}")]
    TexError(#[from] tex::Error),
    #[error("Typst error: {0}")]
    TypstAsLibError(#[from] typst_as_lib::TypstAsLibError),
    #[error("Typst error: {0:?}")]
    TypstError(ecow::vec::EcoVec<typst::diag::SourceDiagnostic>),
    #[error("KaTeX error: {0}")]
    KatexError(#[from] katex::Error),
}

impl TryFrom<Card<Source>> for Card<Rendered> {
    type Error = Error;

    fn try_from(card: Card<Source>) -> Result<Self, Self::Error> {
        Ok(Self {
            term: Rendered {
                html: render(&card.term)?,
                source: card.term,
            },
            definition: Rendered {
                html: render(&card.definition)?,
                source: card.definition,
            },
            topics: card.topics,
        })
    }
}

impl TryFrom<Source> for Rendered {
    type Error = Error;

    fn try_from(source: Source) -> Result<Self, Self::Error> {
        Ok(Rendered {
            html: render(&source)?,
            source,
        })
    }
}

fn render(side: &Source) -> Result<String, Error> {
    match side.format {
        Format::Tex => Ok(tex::render(&side.source)?),
        Format::Markdown => Ok(format!("<div>{}</div>", markdown(&side.source)?)),
        Format::Typst => {
            let font_options = typst_as_lib::typst_kit_options::TypstKitFontOptions::new()
                .include_embedded_fonts(true);

            let engine = typst_as_lib::TypstEngine::builder()
                .main_file(format!(
                    r##"#set page(width: auto, height: auto, margin: 1em)
{}"##,
                    side.source
                ))
                .search_fonts_with(font_options)
                .build();

            let doc = engine.compile::<typst::layout::PagedDocument>().output?;
            let data = typst_svg::svg_merged(&doc, typst::layout::Abs::zero());

            let engine = base64::engine::GeneralPurpose::new(
                &base64::alphabet::STANDARD,
                base64::engine::GeneralPurposeConfig::new(),
            );
            let data = engine.encode(data);

            let mut escaped_source = String::new();
            pulldown_cmark_escape::escape_html(&mut escaped_source, &side.source).unwrap();

            Ok(format!(
                r#"<img src="data:image/svg+xml;base64,{data}" alt="{escaped_source}" title="{escaped_source}" class="w-full h-full typst">"#
            ))
        }
    }
}

pub fn markdown(text: &str) -> katex::Result<String> {
    let scanner = katex_scanner::Scanner::new(text);
    let text = scanner
        .map(|event| event.render())
        .collect::<katex::Result<String>>()?;
    let parser = md::Parser::new_ext(&text, md::Options::ENABLE_TABLES);

    let mut result = String::new();
    md::html::push_html(&mut result, parser);
    Ok(result)
}

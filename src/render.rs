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

pub fn markdown(text: &str) -> String {
    let scanner = katex_scanner::Scanner::new(text);
    let text = scanner.map(|event| event.as_html()).collect::<String>();
    let parser = md::Parser::new_ext(&text, md::Options::ENABLE_TABLES);

    let mut result = String::new();
    md::html::push_html(&mut result, parser);
    result
}

pub mod filters {
    use std::fmt::Display;

    #[allow(clippy::unnecessary_wraps)]
    pub fn markup(text: impl Display) -> askama::Result<String> {
        Ok(super::markdown(&text.to_string()))
    }
}

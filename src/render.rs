use itertools::Itertools;
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

            println!(
                "{}\n{}\n{:?}\n",
                self.input,
                &self.input[self.position..],
                self.state
            );

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
                Self::Text(text) => Ok(text.to_string()),
                Self::Block(content) => katex::render_with_opts(
                    content,
                    Opts::builder()
                        .display_mode(true)
                        .output_type(katex::OutputType::Html)
                        .build()
                        .unwrap(),
                ),
                Self::Inline(content) => katex::render_with_opts(
                    content,
                    Opts::builder()
                        .output_type(katex::OutputType::Html)
                        .build()
                        .unwrap(),
                ),
            }
        }

        pub fn as_html(&self) -> String {
            match self.render() {
                Ok(result) => result,
                Err(err) => {
                    eprintln!("Katex error ignored: {}", err);
                    match self {
                        Self::Text(text) => text.to_string(),
                        Self::Block(content) => format!("$${content}$$"),
                        Self::Inline(content) => format!("${content}$"),
                    }
                }
            }
        }
    }
}

pub fn katex(input: &str) -> String {
    println!("a");
    let scanner = katex_scanner::Scanner::new(input);
    scanner.map(|event| event.as_html()).collect()
}

pub fn markdown(text: &str) -> String {
    let mut result = String::new();
    let parser = md::Parser::new_ext(text, md::Options::ENABLE_TABLES);
    let mut current_text = String::new();
    let mut events = Vec::new();
    for event in parser {
        if let md::Event::Text(text) = event {
            current_text.push_str(text.as_ref());
        } else {
            events.push(md::Event::Text(current_text.clone().into()));
            current_text.clear();
            events.push(event);
        }
    }
    events.push(md::Event::Text(current_text.into()));

    md::html::push_html(
        &mut result,
        events.into_iter().flat_map(|event| match event {
            md::Event::Text(text) if text.contains('$') => {
                std::iter::once(md::Event::Html(katex(&text).into()))
            }
            _ => std::iter::once(event),
        }),
    );
    result
}

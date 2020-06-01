//! Top level docs.

use std::error::Error;
use std::fmt::{self, Write};
use std::fs;
use std::io::Read;

use proc_macro2::TokenTree;
use pulldown_cmark::{Event, Options, Parser, Tag};
use syn::visit::{self, Visit};
use syn::{AttrStyle, ItemFn};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

struct Visitor {
    docs: Vec<Doc>,
}

#[derive(Debug)]
enum Doc {
    FnDoc(FnDoc),
}

impl fmt::Display for Doc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Doc::FnDoc(fn_doc) => write!(f, "{}", fn_doc),
        }
    }
}

/// Does some basic markdown parsing so we can get to the codeblocks.
fn format_markdown(input: &str) -> String {
    let parser = Parser::new_ext(input, Options::empty());

    let mut out = String::new();
    let mut code = String::new();
    let mut is_code = false;

    for event in parser {
        match event {
            Event::Start(tag) => {
                match tag {
                    Tag::CodeBlock { .. } => {
                        // Indented
                        // Fenced(CowStr<'a>)
                        is_code = true
                    }
                    Tag::Paragraph { .. } => write!(code, "\n\n").unwrap(),
                    _ => todo!("{:?}", tag),
                }
            }
            Event::End(tag) => {
                match tag {
                    Tag::CodeBlock { .. } => {
                        // Indented
                        // Fenced(CowStr<'a>)
                        is_code = false;

                        // TODO: Highlight!
                        let ps = SyntaxSet::load_defaults_newlines();
                        let ts = ThemeSet::load_defaults();

                        let syntax = ps.find_syntax_by_extension("rs").unwrap();
                        let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
                        for line in LinesWithEndings::from(&code) {
                            // LinesWithEndings enables use of newlines mode
                            let ranges: Vec<(Style, &str)> = h.highlight(line, &ps);
                            let escaped = as_24_bit_terminal_escaped(&ranges[..], true);
                            write!(out, "{}", escaped).unwrap();
                        }
                        write!(out, "\x1b[0m").unwrap();
                        code.clear();
                    }
                    _ => {}
                }
            }
            Event::Text(text) => {
                if is_code {
                    write!(code, "{}", text).unwrap();
                } else {
                    write!(out, "{}", text).unwrap();
                }
            }
            Event::Code(_) => todo!(),
            Event::Html(_) => todo!(),
            Event::FootnoteReference(_) => todo!(),
            Event::SoftBreak => todo!(),
            Event::HardBreak => todo!(),
            Event::Rule => todo!(),
            Event::TaskListMarker(_) => todo!(),
        }
    }

    write!(out, "\n\n").unwrap();
    out
}

#[derive(Debug)]
struct FnDoc {
    signature: String,
    doc: String,
}

impl fmt::Display for FnDoc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hl = format!("{:-^1$}", "function", 80); // TODO: Useful heading

        write!(
            f,
            "{}\n\n{}{}",
            hl,
            self.signature,
            format_markdown(&self.doc)
        )
    }
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let signature = format!("fn {}()\n\n", &node.sig.ident);

        // The compiler transforms doc comments, such as /// comment and /*! comment */, into
        // attributes before macros are expanded. Each comment is expanded into an attribute of the
        // form #[doc = r"comment"].
        //
        // Outer doc comments like /// Example.
        // Inner doc comments like //! Example.

        let mut doc = String::new();
        for attr in &node.attrs {
            if attr.style == AttrStyle::Outer {
                for token in attr.tokens.clone().into_iter() {
                    match token {
                        TokenTree::Literal(lit) => {
                            let mut lit = lit.to_string();
                            lit.remove(0); // remove the first `"`
                            lit.remove(0); // assume there is a leading space (TODO: Fix this assumption)
                            if !lit.is_empty() {
                                lit.remove(lit.len() - 1); // remove the last `"`
                            }
                            write!(doc, "{}\n", lit).unwrap();
                        }
                        _ => (),
                    }
                }
            }
        }

        self.docs.push(Doc::FnDoc(FnDoc { signature, doc }));

        // Delegate to the default impl to visit any nested functions.
        visit::visit_item_fn(self, node);
    }
}

/// Hello, this is the main doc!
///
///     let mut x = {
///         1 + 1
///     };
fn main() -> Result<(), Box<dyn Error>> {
    let mut file = fs::File::open(
        std::env::args()
            .skip(1)
            .next()
            .expect("no filename provided"),
    )?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let ast = syn::parse_file(&content)?;

    let mut visitor = Visitor { docs: Vec::new() };
    visitor.visit_file(&ast);

    for doc in visitor.docs {
        print!("{}", doc);
    }

    Ok(())
}

use std::path::Path;

use modularize_imports::{modularize_imports, PackageConfig};
use swc_common::{
    errors::{ColorConfig, Handler},
    sync::Lrc,
    BytePos, SourceMap,
};
use swc_core::atoms::Atom;
use swc_css_codegen::writer::basic::{BasicCssWriter, BasicCssWriterConfig};
use swc_ecma_ast::{Module, TaggedTpl, Tpl, TplElement};
use swc_ecma_codegen::{text_writer::JsWriter, Emitter};
use swc_ecma_parser::{lexer::Lexer, EsConfig, Parser, StringInput, Syntax};
use swc_ecma_visit::{Fold, FoldWith};
use swc_html_codegen::writer::basic::{BasicHtmlWriter, BasicHtmlWriterConfig};
use swc_html_minifier::option::MinifyCssOption;

struct TaggedTemplateTransformer {}

impl TaggedTemplateTransformer {
    fn new() -> Self {
        Self {}
    }
}

fn tpl_to_literal_string(tpl: &Tpl) -> Option<String> {
    if !tpl.exprs.is_empty() {
        None
    } else {
        let mut s = String::new();

        for quasi in tpl.quasis.iter() {
            s.push_str(quasi.raw.as_str());
        }

        Some(s)
    }
}

fn replace_tagged_tpl_content_with(n: &TaggedTpl, s: String) -> TaggedTpl {
    TaggedTpl {
        span: n.span,
        tag: n.tag.clone(),
        type_params: n.type_params.clone(),
        tpl: Box::new(Tpl {
            span: n.tpl.span,
            exprs: vec![],
            quasis: vec![TplElement {
                span: n.tpl.span,
                tail: true,
                cooked: None,
                raw: Atom::new(s),
            }],
        }),
    }
}

fn parse_html(code: &str) -> Result<swc_html_ast::DocumentFragment, swc_html_parser::error::Error> {
    let lexer = swc_html_parser::lexer::Lexer::new(swc_common::input::StringInput::new(
        code,
        BytePos(0),
        BytePos(code.len() as u32),
    ));
    let config = swc_html_parser::parser::ParserConfig::default();
    let mut parser = swc_html_parser::parser::Parser::new(lexer, config);
    let context_element = swc_html_ast::Element {
        span: Default::default(),
        namespace: swc_html_ast::Namespace::HTML,
        tag_name: "div".into(),
        attributes: vec![],
        is_self_closing: false,
        children: vec![],
        content: None,
    };
    parser.parse_document_fragment(context_element, swc_html_ast::DocumentMode::NoQuirks, None)
}

fn minify_html(code: &str) -> String {
    match parse_html(code) {
        Ok(mut doc) => {
            let config = swc_html_minifier::option::MinifyOptions {
                force_set_html5_doctype: false,
                collapse_whitespaces: swc_html_minifier::option::CollapseWhitespaces::All,
                remove_empty_metadata_elements: true,
                remove_comments: true,
                preserve_comments: None,
                minify_conditional_comments: true,
                remove_empty_attributes: false,
                remove_redundant_attributes:
                    swc_html_minifier::option::RemoveRedundantAttributes::None,
                collapse_boolean_attributes: false,
                merge_metadata_elements: true,
                normalize_attributes: true,
                minify_json: swc_html_minifier::option::MinifyJsonOption::Bool(true),
                minify_js: swc_html_minifier::option::MinifyJsOption::Bool(true),
                minify_css: MinifyCssOption::Bool(true),
                minify_additional_scripts_content: None,
                minify_additional_attributes: None,
                sort_space_separated_attribute_values: false,
                sort_attributes: false,
            };
            let context_element = swc_html_ast::Element {
                span: Default::default(),
                namespace: swc_html_ast::Namespace::HTML,
                tag_name: "div".into(),
                attributes: vec![],
                is_self_closing: false,
                children: vec![],
                content: None,
            };
            swc_html_minifier::minify_document_fragment(&mut doc, &context_element, &config);
            document_to_html_string(&doc)
        }

        Err(err) => {
            eprintln!("Error parsing html: {err:?}\n{code}");
            code.to_string()
        }
    }
}

fn document_to_html_string(document: &swc_html_ast::DocumentFragment) -> String {
    let mut html_str = String::new();
    {
        use swc_html_codegen::Emit;
        let wr = BasicHtmlWriter::new(&mut html_str, None, BasicHtmlWriterConfig::default());
        let mut gen = swc_html_codegen::CodeGenerator::new(
            wr,
            swc_html_codegen::CodegenConfig {
                scripting_enabled: true,
                minify: false,
                ..Default::default()
            },
        );

        gen.emit(&document).unwrap();
    }

    html_str
}

fn document_to_css_string(document: &swc_css_ast::Stylesheet) -> String {
    let mut css_str = String::new();
    {
        use swc_css_codegen::Emit;
        let wr = BasicCssWriter::new(&mut css_str, None, BasicCssWriterConfig::default());
        let mut gen = swc_css_codegen::CodeGenerator::new(
            wr,
            swc_css_codegen::CodegenConfig { minify: true },
        );

        gen.emit(&document).unwrap();
    }

    css_str
}

fn parse_css(code: &str) -> Result<swc_css_ast::Stylesheet, swc_css_parser::error::Error> {
    let config = swc_css_parser::parser::ParserConfig::default();
    let lexer = swc_css_parser::lexer::Lexer::new(
        swc_common::input::StringInput::new(code, BytePos(0), BytePos(code.len() as u32)),
        None,
        config,
    );
    let mut parser = swc_css_parser::parser::Parser::new(lexer, config);
    parser.parse_all()
}

fn minify_css(code: &str) -> String {
    match parse_css(code) {
        Ok(doc) => document_to_css_string(&doc),

        Err(err) => {
            eprintln!("Error parsing html: {err:?}\n{code}");
            code.to_string()
        }
    }
}

impl Fold for TaggedTemplateTransformer {
    fn fold_tagged_tpl(&mut self, n: TaggedTpl) -> TaggedTpl {
        let tag = if let Some(id) = n.tag.as_ident() {
            id.sym.as_str()
        } else {
            ""
        };

        if tag == "html" {
            if let Some(content) = tpl_to_literal_string(&n.tpl) {
                let new_content = minify_html(&content);
                return replace_tagged_tpl_content_with(&n, new_content);
            } else {
                println!("can't handle html with exprs");
            }
        } else if tag == "css" {
            if let Some(content) = tpl_to_literal_string(&n.tpl) {
                let new_content = minify_css(&content);
                return replace_tagged_tpl_content_with(&n, new_content);
            } else {
                println!("can't handle css with exprs");
            }
        } else {
            println!("ignoring tagged template {n:?}");
        }
        n.fold_children_with(self)
    }
}

fn main() {
    // https://github.com/swc-project/plugins/blob/main/packages/transform-imports/transform/tests/fixture.rs
    let mut transform_importmaps = modularize_imports(modularize_imports::Config {
        packages: vec![(
            "mod1".to_string(),
            PackageConfig {
                transform: "deps/mod1.js".into(),
                prevent_full_import: false,
                skip_default_conversion: true,
            },
        )]
        .into_iter()
        .collect(),
    });

    let mut transform_tagged_tpls = TaggedTemplateTransformer::new();

    let cm: Lrc<SourceMap> = Default::default();
    let mod_0 = parse_module(Path::new("mymod.js"), &cm);
    let mod_1 = transform_importmaps.fold_module(mod_0);
    let mod_2 = transform_tagged_tpls.fold_module(mod_1);
    let new_module_code = format_module(mod_2, &cm);
    println!("{new_module_code}");
}

fn parse_module(path: &Path, cm: &Lrc<SourceMap>) -> Module {
    let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));

    let fm = cm.load_file(path).expect("failed to load js file");

    let syntax = Syntax::Es(EsConfig {
        jsx: true,
        ..Default::default()
    });

    let lexer = Lexer::new(
        syntax,
        // EsVersion defaults to es5
        Default::default(),
        StringInput::from(&*fm),
        None,
    );

    let mut parser = Parser::new_from(lexer);

    for e in parser.take_errors() {
        e.into_diagnostic(&handler).emit();
    }

    parser
        .parse_module()
        .map_err(|e| {
            // Unrecoverable fatal error occurred
            e.into_diagnostic(&handler).emit()
        })
        .expect("failed to parser module")
}

fn format_module(module: Module, cm: &Lrc<SourceMap>) -> String {
    let mut buf = vec![];

    {
        let mut emitter = Emitter {
            cfg: Default::default(),
            comments: None,
            cm: cm.clone(),
            wr: JsWriter::new(cm.clone(), "\n", &mut buf, None),
        };

        // Write the module to the buffer
        emitter.emit_module(&module).expect("Failed to emit module");
    }

    // Convert the buffer into a String
    String::from_utf8(buf).expect("Buffer contains invalid UTF-8")
}

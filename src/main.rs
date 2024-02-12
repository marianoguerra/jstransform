use std::path::Path;

use modularize_imports::{modularize_imports, PackageConfig};
use swc_common::sync::Lrc;
use swc_common::{
    errors::{ColorConfig, Handler},
    SourceMap,
};
use swc_ecma_ast::Module;
use swc_ecma_codegen::text_writer::JsWriter;
use swc_ecma_codegen::Emitter;
use swc_ecma_parser::EsConfig;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};
use swc_ecma_visit::Fold;

fn main() {
    // https://github.com/swc-project/plugins/blob/main/packages/transform-imports/transform/tests/fixture.rs
    let mut transform = modularize_imports(modularize_imports::Config {
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

    let cm: Lrc<SourceMap> = Default::default();
    let module = parse_module(Path::new("mymod.js"), &cm);
    let new_module = transform.fold_module(module);
    let new_module_code = format_module(new_module, &cm);
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

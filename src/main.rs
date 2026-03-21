/// KelpyShark CLI — the `kelpy` command.
///
/// Usage:
///   kelpy run <file.ks>    - Execute a KelpyShark script
///   kelpy repl              - Start the interactive REPL
///   kelpy build <file.ks>   - Compile to a target language
///   kelpy new <name>        - Create a new KelpyShark project

use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::process;

use kelpyshark_interpreter::interpreter::Interpreter;

#[derive(Parser)]
#[command(name = "kelpy")]
#[command(version = "0.1.0")]
#[command(about = "🦈 The KelpyShark Programming Language")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a KelpyShark script
    Run {
        /// Path to the .ks file
        file: PathBuf,
    },
    /// Start the interactive REPL
    Repl,
    /// Compile a KelpyShark file to a target language
    Build {
        /// Path to the .ks file
        file: PathBuf,
        /// Target language: c, js, java, cs
        #[arg(short, long, default_value = "c")]
        target: String,
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Compile a KelpyShark file (alias for build)
    Compile {
        /// Path to the .ks file
        file: PathBuf,
        /// Target language: c, js, java, cs
        #[arg(short, long, default_value = "c")]
        target: String,
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Create a new KelpyShark project
    New {
        /// Project name
        name: String,
    },
    /// Install a KelpyShark package
    Install {
        /// Package name
        package: String,
    },
    /// Publish a KelpyShark package
    Publish,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file } => cmd_run(&file),
        Commands::Repl => cmd_repl(),
        Commands::Build {
            file,
            target,
            output,
        }
        | Commands::Compile {
            file,
            target,
            output,
        } => cmd_build(&file, &target, output.as_deref()),
        Commands::New { name } => cmd_new(&name),
        Commands::Install { package } => cmd_install(&package),
        Commands::Publish => cmd_publish(),
    }
}

fn cmd_run(file: &PathBuf) {
    let source = match fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", file.display(), e);
            process::exit(1);
        }
    };

    let mut interp = Interpreter::new();
    if let Err(e) = interp.exec(&source) {
        eprintln!("{}", e);
        process::exit(1);
    }
}

fn cmd_repl() {
    let mut interp = Interpreter::new();
    interp.repl();
}

fn cmd_build(file: &PathBuf, target: &str, output: Option<&std::path::Path>) {
    let source = match fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", file.display(), e);
            process::exit(1);
        }
    };

    // Parse the source to AST
    let mut lexer = kelpyshark_compiler::lexer::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    let mut parser = kelpyshark_compiler::parser::Parser::new(tokens);
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    // Determine default extension based on target
    let default_ext = match target {
        "c" => "c",
        "js" | "javascript" => "js",
        "java" => "java",
        "cs" | "csharp" => "cs",
        _ => "out",
    };

    let default_output_name = file
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let default_output = PathBuf::from(format!("{}.{}", default_output_name, default_ext));
    let output_path = output.unwrap_or(&default_output);

    let generated = match target {
        "c" => {
            kelpyshark_compiler::codegen::c::generate(&program)
        }
        "js" | "javascript" => {
            kelpyshark_compiler::codegen::javascript::generate(&program)
        }
        "java" => {
            eprintln!("🦈 Java code generation is not yet implemented.");
            process::exit(1);
        }
        "cs" | "csharp" => {
            eprintln!("🦈 C# code generation is not yet implemented.");
            process::exit(1);
        }
        other => {
            eprintln!("Unknown target: '{}'. Supported: c, js, java, cs", other);
            process::exit(1);
        }
    };

    if let Err(e) = fs::write(output_path, &generated) {
        eprintln!("Error writing output '{}': {}", output_path.display(), e);
        process::exit(1);
    }

    println!(
        "🦈 Compiled {} → {} ({} target, {} bytes)",
        file.display(),
        output_path.display(),
        target,
        generated.len()
    );
}

fn cmd_new(name: &str) {
    let base = PathBuf::from(name);

    // Create project structure
    let dirs = [
        base.join("src"),
        base.join("libs"),
    ];

    for dir in &dirs {
        if let Err(e) = fs::create_dir_all(dir) {
            eprintln!("Error creating directory '{}': {}", dir.display(), e);
            process::exit(1);
        }
    }

    // Create kelpy.toml
    let toml_content = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
description = ""

[dependencies]
"#,
        name
    );
    fs::write(base.join("kelpy.toml"), toml_content).unwrap();

    // Create main.ks
    let main_content = r#"# Welcome to KelpyShark!
print "Hello, KelpyShark! 🦈"
"#;
    fs::write(base.join("src").join("main.ks"), main_content).unwrap();

    println!("🦈 Created new KelpyShark project '{}'", name);
    println!("   {}/", name);
    println!("   ├── kelpy.toml");
    println!("   ├── src/");
    println!("   │   └── main.ks");
    println!("   └── libs/");
}

fn cmd_install(package: &str) {
    println!(
        "🦈 Package manager not yet implemented. Would install: {}",
        package
    );
}

fn cmd_publish() {
    println!("🦈 Package publishing not yet implemented.");
}

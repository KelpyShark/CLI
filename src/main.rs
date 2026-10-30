/// KelpyShark CLI — the `kelpy` command.
///
/// Usage:
///   kelpy run <file.ks>    - Execute a KelpyShark script
///   kelpy repl              - Start the interactive REPL
///   kelpy build <file.ks>   - Compile to a target language
///   kelpy new <name>        - Create a new KelpyShark project
///   kelpy install <package> - Install a package
///   kelpy publish           - Publish the current package
///   kelpy update            - Update all installed packages

use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::process;

use kelpyshark_interpreter::interpreter::Interpreter;
use kelpyshark_package_manager::registry::Registry;

#[derive(Parser)]
#[command(name = "kelpy")]
#[command(version = "0.2.0")]
#[command(about = "The KelpyShark Programming Language ")]
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
        /// Arguments passed to the script after `--`
        #[arg(last = true, num_args = 0.., allow_hyphen_values = true)]
        args: Vec<String>,
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
        /// Package name (optional — installs all deps from kelpy.toml if omitted)
        package: Option<String>,
    },
    /// Publish the current package to the local registry
    Publish,
    /// Update all installed packages to their latest versions
    Update,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file, args: _ } => cmd_run(&file),
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
        Commands::Install { package } => cmd_install(package.as_deref()),
        Commands::Publish => cmd_publish(),
        Commands::Update => cmd_update(),
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

    let mut interp = create_interpreter();
    if let Err(e) = interp.exec(&source) {
        eprintln!("{}", e);
        process::exit(1);
    }
}

fn cmd_repl() {
    let mut interp = create_interpreter();
    interp.repl();
}

fn create_interpreter() -> Interpreter {
    let mut interp = Interpreter::new();
    for (name, module) in kelpyshark_stdlib::modules() {
        interp.register_module(name, module);
    }
    interp
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

    if let Err(e) = kelpyshark_compiler::semantic::SemanticAnalyzer::check(&program) {
        eprintln!("{}", e);
        process::exit(1);
    }

    // Determine default extension based on target
    let default_ext = match target {
        "c" => "c",
        "js" | "javascript" => "js",
        "java" => "java",
        "cs" | "csharp" => "cs",
        _ => "out",
    };

    let default_output = default_build_output(file, default_ext);
    let output_path = output.unwrap_or(&default_output);

    let generated = match target {
        "c" => match kelpyshark_compiler::codegen::c::generate_checked(&program) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("C target error: {}", error);
                process::exit(1);
            }
        },
        "js" | "javascript" => {
            match kelpyshark_compiler::codegen::javascript::generate_checked(&program) {
                Ok(source) => source,
                Err(error) => {
                    eprintln!("JavaScript target error: {}", error);
                    process::exit(1);
                }
            }
        }
        "java" => match kelpyshark_compiler::codegen::java::generate_checked(&program) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("Java target error: {}", error);
                process::exit(1);
            }
        },
        "cs" | "csharp" => {
            match kelpyshark_compiler::codegen::cs::generate_checked(&program) {
                Ok(source) => source,
                Err(error) => {
                    eprintln!("C# target error: {}", error);
                    process::exit(1);
                }
            }
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
        "Compiled {} → {} ({} target, {} bytes)",
        file.display(),
        output_path.display(),
        target,
        generated.len()
    );
}

fn default_build_output(file: &std::path::Path, extension: &str) -> PathBuf {
    file.with_extension(extension)
}

fn cmd_new(name: &str) {
    if let Err(error) = kelpyshark_package_manager::manifest::validate_package_name(name) {
        eprintln!("Invalid project name: {}", error);
        process::exit(1);
    }

    let base = PathBuf::from(name);

    if base.exists() {
        eprintln!("Project path '{}' already exists", base.display());
        process::exit(1);
    }
    if let Err(error) = fs::create_dir(&base) {
        eprintln!("Error creating project directory '{}': {}", base.display(), error);
        process::exit(1);
    }
    for dir in [base.join("src"), base.join("libs")] {
        if let Err(error) = fs::create_dir(&dir) {
            eprintln!("Error creating directory '{}': {}", dir.display(), error);
            process::exit(1);
        }
    }

    // Create kelpy.toml
    let toml_content = format!(
        r#"[package]
name = "{}"
version = "0.2.0"
description = ""

[dependencies]
"#,
        name
    );
    if let Err(error) = fs::write(base.join("kelpy.toml"), toml_content) {
        eprintln!("Error writing project manifest: {}", error);
        process::exit(1);
    }

    // Create main.ks
    let main_content = r#"# Welcome to KelpyShark!
print "Hello, KelpyShark! "
"#;
    if let Err(error) = fs::write(base.join("src").join("main.ks"), main_content) {
        eprintln!("Error writing starter program: {}", error);
        process::exit(1);
    }

    println!("Created new KelpyShark project '{}'", name);
    println!("   {}/", name);
    println!("   ├── kelpy.toml");
    println!("   ├── src/");
    println!("   │   └── main.ks");
    println!("   └── libs/");
}

fn cmd_install(package: Option<&str>) {
    let registry = Registry::default();
    if let Err(e) = registry.init() {
        eprintln!("Registry error: {}", e);
        process::exit(1);
    }

    let project_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    match package {
        Some(pkg) => {
            // Install a specific package by name
            match kelpyshark_package_manager::installer::install_one(&project_dir, pkg, &registry) {
                Ok(msg) => println!("Installed {}", msg),
                Err(e)  => {
                    eprintln!("Install error: {}", e);
                    process::exit(1);
                }
            }
        }
        None => {
            // Install all dependencies from kelpy.toml
            match kelpyshark_package_manager::installer::install_all(&project_dir, &registry) {
                Ok(installed) => {
                    if installed.is_empty() {
                        println!("Nothing to install.");
                    } else {
                        for pkg in &installed {
                            println!("Installed {}", pkg);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Install error: {}", e);
                    process::exit(1);
                }
            }
        }
    }
}

fn cmd_publish() {
    let registry = Registry::default();
    if let Err(e) = registry.init() {
        eprintln!("Registry error: {}", e);
        process::exit(1);
    }

    let project_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    match kelpyshark_package_manager::publisher::publish(&project_dir, &registry) {
        Ok(msg) => println!("{}", msg),
        Err(e)  => {
            eprintln!("Publish error: {}", e);
            process::exit(1);
        }
    }
}

fn cmd_update() {
    let registry = Registry::default();
    if let Err(e) = registry.init() {
        eprintln!("Registry error: {}", e);
        process::exit(1);
    }

    let project_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    match kelpyshark_package_manager::installer::update_all(&project_dir, &registry) {
        Ok(updated) => {
            if updated.is_empty() {
                println!("All packages are up to date.");
            } else {
                for msg in &updated {
                    println!("Updated {}", msg);
                }
            }
        }
        Err(e) => {
            eprintln!("Update error: {}", e);
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_accepts_script_arguments_after_separator() {
        let cli = Cli::try_parse_from([
            "kelpyshark",
            "run",
            "main.ks",
            "--",
            "Alice",
            "--verbose",
        ])
        .expect("run should accept script arguments after --");

        match cli.command {
            Commands::Run { file, args } => {
                assert_eq!(file, PathBuf::from("main.ks"));
                assert_eq!(args, vec!["Alice", "--verbose"]);
            }
            _ => panic!("expected the run command"),
        }
    }

    #[test]
    fn default_build_output_stays_beside_the_input_file() {
        let source = PathBuf::from("examples/hello.ks");
        assert_eq!(
            default_build_output(&source, "java"),
            PathBuf::from("examples/hello.java")
        );
    }
}

//! Command-line interface for PreFigure, shadowing prefig/cli.py.

use clap::{Parser, Subcommand};
use prefig_core::evaluator::ExpressionContext;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "prefig", version, about = "PreFigure: an authoring system for mathematical diagrams")]
struct Cli {
    /// -v for information and -vv for debugging
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Build a PreFigure diagram from source
    Build {
        /// Desired output format: 'svg' (default), 'svg11' (SVG 1.1), or 'tactile'
        #[arg(short, long, default_value = "svg")]
        format: String,
        /// Location of a publication file
        #[arg(short, long)]
        publication: Option<String>,
        /// Ignore any publication file
        #[arg(short = 'i', long)]
        ignore_publication: bool,
        /// Suppress the caption when creating tactile diagrams
        #[arg(short = 's', long)]
        suppress_caption: bool,
        filename: String,
    },
    /// Convert the PreFigure SVG into a PDF
    Pdf {
        /// Resolution for the conversion (tactile diagrams require 72)
        #[arg(short, long, default_value_t = 72)]
        dpi: u32,
        /// Build from PreFigure source before converting
        #[arg(short = 'b', long)]
        build_first: bool,
        /// Output format, if building from source
        #[arg(short, long, default_value = "svg")]
        format: String,
        /// Location of a publication file, if building from source
        #[arg(short, long)]
        publication: Option<String>,
        /// Ignore any publication file, if building from source
        #[arg(short = 'i', long)]
        ignore_publication: bool,
        filename: String,
    },
    /// Convert the PreFigure SVG into a PNG
    Png {
        /// Build from PreFigure source before converting
        #[arg(short = 'b', long)]
        build_first: bool,
        /// Output format, if building from source
        #[arg(short, long, default_value = "svg")]
        format: String,
        /// Location of a publication file, if building from source
        #[arg(short, long)]
        publication: Option<String>,
        /// Ignore any publication file, if building from source
        #[arg(short = 'i', long)]
        ignore_publication: bool,
        filename: String,
    },
    /// Set up a new PreFigure project
    New,
    /// Initialize the local installation of PreFigure
    Init,
    /// Install PreFigure examples in the current directory
    Examples,
    /// Check that a source file is valid
    Validate { filename: String },
    /// Evaluate a PreFigure expression and print the result
    Eval { expression: String },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    init_logging(cli.verbose);

    let result: Result<(), String> = match cli.command {
        Command::Eval { expression } => {
            let mut ctx = ExpressionContext::new();
            match ctx.valid_eval(&expression) {
                Ok(value) => {
                    println!("{value:?}");
                    Ok(())
                }
                Err(e) => Err(e.to_string()),
            }
        }
        Command::Build {
            format,
            publication,
            ignore_publication,
            suppress_caption,
            filename,
        } => prefig_core::engine::build(
            &format,
            &filename,
            publication.as_deref(),
            ignore_publication,
            suppress_caption,
        )
        .map(|_| ()),
        Command::Pdf {
            dpi,
            build_first,
            format,
            publication,
            ignore_publication,
            filename,
        } => prefig_core::engine::pdf(
            &format,
            &filename,
            build_first,
            publication.as_deref(),
            ignore_publication,
            dpi,
        ),
        Command::Png {
            build_first,
            format,
            publication,
            ignore_publication,
            filename,
        } => prefig_core::engine::png(
            &format,
            &filename,
            build_first,
            publication.as_deref(),
            ignore_publication,
        ),
        Command::New => prefig_core::engine::new_project(),
        Command::Init => prefig_core::engine::init(),
        Command::Examples => prefig_core::engine::examples(),
        Command::Validate { filename } => prefig_core::engine::validate_source(&filename),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Route log records to stderr; -v enables info, -vv debug.
fn init_logging(verbose: u8) {
    let level = match verbose {
        0 => log::LevelFilter::Warn,
        1 => log::LevelFilter::Info,
        _ => log::LevelFilter::Debug,
    };
    let _ = env_logger::Builder::new()
        .filter_level(level)
        .format_timestamp(None)
        .try_init();
}

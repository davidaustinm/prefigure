//! Port of prefig/engine.py: entry points for building diagrams.

use crate::core::label::LabelState;
use crate::core::parse::{check_duplicate_handles, find_diagrams, mk_diagram};
use crate::xml::El;
use std::collections::HashSet;

/// Build a diagram from source text. Mirrors engine.build_from_string: the
/// first <diagram> in the document is built; returns (svg, annotations).
#[cfg(feature = "xml-parse")]
pub fn build_from_string(
    format: &str,
    input_string: &str,
    environment: &str,
    labels: LabelState,
) -> Result<(String, Option<String>), String> {
    build_source(format, input_string, "prefig", environment, labels)
}

/// Like build_from_string but with control over the filename used for id
/// prefixes (the Python pipeline derives ids from the source file's name).
#[cfg(feature = "xml-parse")]
pub fn build_source(
    format: &str,
    input_string: &str,
    filename: &str,
    environment: &str,
    labels: LabelState,
) -> Result<(String, Option<String>), String> {
    build_source_with(
        format,
        input_string,
        filename,
        environment,
        labels,
        None,
        false,
    )
}

/// Full build entry point: publication defaults and caption suppression, as the
/// CLI/PreTeXt pipelines need.
#[cfg(feature = "xml-parse")]
pub fn build_source_with(
    format: &str,
    input_string: &str,
    filename: &str,
    environment: &str,
    labels: LabelState,
    publication: Option<El>,
    suppress_caption: bool,
) -> Result<(String, Option<String>), String> {
    let tree = crate::xml::parse_str(input_string)?;
    let diagrams = find_diagrams(&tree);
    let Some(diagram) = diagrams.first() else {
        return Err("no <diagram> element found".to_string());
    };

    check_duplicate_handles(diagram, &mut HashSet::new());

    mk_diagram(
        diagram,
        format,
        publication,
        filename,
        suppress_caption,
        None, // diagram number
        environment,
        labels,
    )
}

/// Build from a file like the CLI does: writes output/<stem>.svg (and the
/// annotations XML) next to the source. Mirrors engine.build: resolves a
/// publication file (unless ignored) and honours suppress_caption. Returns the
/// (possibly suffix-corrected) source path, like Python.
#[cfg(all(feature = "xml-parse", not(target_arch = "wasm32")))]
pub fn build(
    format: &str,
    filename: &str,
    publication: Option<&str>,
    ignore_publication: bool,
    suppress_caption: bool,
) -> Result<std::path::PathBuf, String> {
    use std::path::{Path, PathBuf};

    // add a .xml suffix if none was given
    let mut source_path = PathBuf::from(filename);
    if source_path.extension().is_none() {
        source_path.set_extension("xml");
    }

    let publication = resolve_publication(publication, ignore_publication);

    let source =
        std::fs::read_to_string(&source_path).map_err(|e| format!("reading {filename}: {e}"))?;
    let stem = source_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "prefig".to_string());

    let labels = LabelState::local(format);
    let (svg, annotations) = build_source_with(
        format,
        &source,
        &stem,
        "pf_cli",
        labels,
        publication,
        suppress_caption,
    )?;

    let output_dir = source_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("output");
    std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;
    let svg_path = output_dir.join(format!("{stem}.svg"));
    std::fs::write(&svg_path, svg).map_err(|e| e.to_string())?;
    if let Some(annotations) = annotations {
        let annotations_path = output_dir.join(format!("{stem}-annotations.xml"));
        std::fs::write(annotations_path, annotations).map_err(|e| e.to_string())?;
    }
    log::info!("Wrote {}", svg_path.display());
    Ok(source_path)
}

/// Locate and load a publication file, mirroring engine.build's search: look
/// for the named file (or `pf_publication.xml`) in the working directory and
/// its parents. Returns the `<prefigure>` element to use as diagram defaults.
#[cfg(all(feature = "xml-parse", not(target_arch = "wasm32")))]
fn resolve_publication(publication: Option<&str>, ignore_publication: bool) -> Option<El> {
    if ignore_publication && publication.is_none() {
        return None;
    }
    let pub_requested = !ignore_publication && publication.is_some();
    let pub_name = publication.unwrap_or("pf_publication.xml");

    let cwd = std::env::current_dir().ok()?;
    let mut dir = Some(cwd);
    let mut found = None;
    while let Some(cur) = dir {
        let candidate = cur.join(pub_name);
        if candidate.exists() {
            found = Some(candidate);
            break;
        }
        dir = cur.parent().map(|p| p.to_path_buf());
    }

    match found {
        Some(path) => {
            log::info!("Applying PreFigure publication file {}", path.display());
            load_publication(&path)
        }
        None => {
            if pub_requested {
                log::warn!("PreFigure publication file not found");
            }
            None
        }
    }
}

/// Parse a publication file and return its `<prefigure>` element (namespaces are
/// stripped by the parser), whose children become diagram defaults.
#[cfg(all(feature = "xml-parse", not(target_arch = "wasm32")))]
fn load_publication(path: &std::path::Path) -> Option<El> {
    let content = std::fs::read_to_string(path).ok()?;
    let tree = crate::xml::parse_str(&content).ok()?;
    if tree.borrow().tag == "prefigure" {
        return Some(tree);
    }
    crate::xml::find_descendants(&tree, "prefigure")
        .into_iter()
        .next()
}

/// Convert a built SVG to PDF via rsvg-convert. Mirrors engine.pdf.
#[cfg(all(feature = "xml-parse", not(target_arch = "wasm32")))]
pub fn pdf(
    format: &str,
    filename: &str,
    build_first: bool,
    publication: Option<&str>,
    ignore_publication: bool,
    dpi: u32,
) -> Result<(), String> {
    let svg_path = resolve_svg_path(
        format,
        filename,
        build_first,
        publication,
        ignore_publication,
    )?;
    let output_file = svg_path.with_extension("pdf");
    let dpi = dpi.to_string();
    log::info!("Converting {} to PDF", svg_path.display());
    run_rsvg(&[
        "-a",
        "-d",
        &dpi,
        "-p",
        &dpi,
        "-f",
        "pdf",
        "-o",
        &output_file.to_string_lossy(),
        &svg_path.to_string_lossy(),
    ])
}

/// Convert a built SVG to PNG (300 dpi) via rsvg-convert. Mirrors engine.png.
#[cfg(all(feature = "xml-parse", not(target_arch = "wasm32")))]
pub fn png(
    format: &str,
    filename: &str,
    build_first: bool,
    publication: Option<&str>,
    ignore_publication: bool,
) -> Result<(), String> {
    let svg_path = resolve_svg_path(
        format,
        filename,
        build_first,
        publication,
        ignore_publication,
    )?;
    let output_file = svg_path.with_extension("png");
    log::info!("Converting {} to PNG", svg_path.display());
    run_rsvg(&[
        "-a",
        "-d",
        "300",
        "-p",
        "300",
        "-f",
        "png",
        "-o",
        &output_file.to_string_lossy(),
        &svg_path.to_string_lossy(),
    ])
}

/// Find the SVG to convert: build it first, or locate an existing one in the
/// working directory, an `output/` subdirectory, or by descending the tree.
#[cfg(all(feature = "xml-parse", not(target_arch = "wasm32")))]
fn resolve_svg_path(
    format: &str,
    filename: &str,
    build_first: bool,
    publication: Option<&str>,
    ignore_publication: bool,
) -> Result<std::path::PathBuf, String> {
    use std::path::PathBuf;

    if build_first {
        let source_path = build(format, filename, publication, ignore_publication, false)?;
        let stem = source_path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "prefig".to_string());
        return Ok(source_path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("output")
            .join(format!("{stem}.svg")));
    }

    let mut svg = PathBuf::from(filename);
    svg.set_extension("svg");
    let name = svg
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();

    // current directory, then an output/ subdirectory
    if svg.exists() {
        return Ok(svg);
    }
    let in_output = std::path::Path::new("output").join(&name);
    if in_output.exists() {
        return Ok(in_output);
    }
    // finally, descend the working directory
    if let Some(found) = find_file(&std::env::current_dir().map_err(|e| e.to_string())?, &name) {
        return Ok(found);
    }
    Err(format!("Unable to find {}", svg.display()))
}

#[cfg(all(feature = "xml-parse", not(target_arch = "wasm32")))]
fn find_file(dir: &std::path::Path, name: &std::ffi::OsStr) -> Option<std::path::PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut subdirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        // Only descend into real directories. `file_type()` reports the entry
        // itself, not a symlink's target, so a symlink pointing at an ancestor
        // is treated as a non-directory here and never recursed into — which
        // avoids an infinite loop and matches Python's os.walk default
        // (followlinks=False).
        let is_real_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if is_real_dir {
            subdirs.push(path);
        } else if path.file_name() == Some(name) {
            return Some(path);
        }
    }
    for sub in subdirs {
        if let Some(found) = find_file(&sub, name) {
            return Some(found);
        }
    }
    None
}

#[cfg(all(feature = "xml-parse", not(target_arch = "wasm32")))]
fn run_rsvg(args: &[&str]) -> Result<(), String> {
    if which("rsvg-convert").is_none() {
        return Err("rsvg-convert is required for PDF/PNG conversion. \
             See the installation instructions at https://prefigure.org"
            .to_string());
    }
    let status = std::process::Command::new("rsvg-convert")
        .args(args)
        .status()
        .map_err(|e| format!("running rsvg-convert: {e}"))?;
    if !status.success() {
        return Err("rsvg-convert failed".to_string());
    }
    Ok(())
}

/// Minimal `shutil.which`: is an executable on PATH?
#[cfg(all(feature = "xml-parse", not(target_arch = "wasm32")))]
pub(crate) fn which(program: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(program);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

// ---------- resource-management subcommands (cli.py) ----------
//
// These mirror the Python CLI's project scaffolding. They operate on the
// PreFigure resource tree (`prefig/resources`), located next to the Python
// package: via the PREFIG_RESOURCES env var, by walking up from the working
// directory, or relative to this crate in the repo.

/// Locate the `prefig/resources` directory.
#[cfg(all(feature = "xml-parse", not(target_arch = "wasm32")))]
pub fn find_resources_dir() -> Option<std::path::PathBuf> {
    if let Ok(dir) = std::env::var("PREFIG_RESOURCES") {
        let path = std::path::PathBuf::from(dir);
        if path.is_dir() {
            return Some(path);
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let mut dir = Some(cwd);
        while let Some(cur) = dir {
            let candidate = cur.join("prefig/resources");
            if candidate.is_dir() {
                return Some(candidate);
            }
            dir = cur.parent().map(|p| p.to_path_buf());
        }
    }
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prefig/resources");
    if repo.is_dir() {
        return Some(repo);
    }
    None
}

/// Recursively copy `src` into `dst`, merging into existing directories
/// (Python's shutil.copytree(dirs_exist_ok=True)).
#[cfg(all(feature = "xml-parse", not(target_arch = "wasm32")))]
fn copy_tree(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())?.flatten() {
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_tree(&from, &to)?;
        } else {
            std::fs::copy(&from, &to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(all(feature = "xml-parse", not(target_arch = "wasm32")))]
fn resources() -> Result<std::path::PathBuf, String> {
    find_resources_dir().ok_or_else(|| {
        "Could not find the PreFigure resource files. Set PREFIG_RESOURCES or run \
         from a PreFigure checkout."
            .to_string()
    })
}

/// `prefig init`: install the MathJax bundle and Braille29 font, mirroring
/// cli.py's init. MathJax is fetched with `npm install`.
#[cfg(all(feature = "xml-parse", not(target_arch = "wasm32")))]
pub fn init() -> Result<(), String> {
    use std::path::Path;
    let resources = resources()?;

    // MathJax bundle: resources/js -> prefig/core/mj_sre
    let destination = resources
        .parent()
        .unwrap_or(Path::new("."))
        .join("core")
        .join("mj_sre");
    log::info!("Installing MathJax libraries in {}", destination.display());
    copy_tree(&resources.join("js"), &destination)?;
    let _ = std::fs::remove_dir_all(destination.join("node_modules"));

    match which("npm") {
        Some(npm) => {
            let status = std::process::Command::new(npm)
                .arg("install")
                .current_dir(&destination)
                .status();
            if status.map(|s| !s.success()).unwrap_or(true) {
                log::warn!("MathJax installation failed. Is npm installed on your system?");
            }
        }
        None => log::error!("Cannot find npm to install MathJax for PreFigure"),
    }

    // Braille29 font: resources/fonts -> ~/.fonts
    if let Some(home) = home_dir() {
        log::info!("Installing the Braille29 font");
        copy_tree(&resources.join("fonts"), &home.join(".fonts"))?;
        if let Some(fc_cache) = which("fc-cache") {
            let _ = std::process::Command::new(fc_cache).arg("-f").status();
        } else {
            log::warn!("Unable to run fc-cache to install the Braille29 font");
        }
    }

    log::info!("PreFigure initialization is complete");
    Ok(())
}

/// `prefig new`: scaffold a new project (diagcess tools + template + source/).
#[cfg(all(feature = "xml-parse", not(target_arch = "wasm32")))]
pub fn new_project() -> Result<(), String> {
    let resources = resources()?;
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    log::info!("Setting up new PreFigure project");
    copy_tree(&resources.join("diagcess"), &cwd)?;
    copy_tree(&resources.join("template"), &cwd)?;
    std::fs::create_dir_all(cwd.join("source")).map_err(|e| e.to_string())?;
    Ok(())
}

/// `prefig examples`: install the bundled examples into ./examples.
#[cfg(all(feature = "xml-parse", not(target_arch = "wasm32")))]
pub fn examples() -> Result<(), String> {
    let resources = resources()?;
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    log::info!("Installing PreFigure examples into {}", cwd.display());
    copy_tree(&resources.join("examples"), &cwd.join("examples"))?;
    Ok(())
}

/// `prefig validate`: validate a source file against the RelaxNG schema. There
/// is no mature pure-Rust RelaxNG validator, so this shells out to `xmllint` or
/// `jing` if one is available, using the bundled schema.
#[cfg(all(feature = "xml-parse", not(target_arch = "wasm32")))]
pub fn validate_source(xml_file: &str) -> Result<(), String> {
    let schema = resources()?.join("schema").join("pf_schema.rng");
    if !schema.exists() {
        return Err(format!(
            "PreFigure schema not found at {}",
            schema.display()
        ));
    }
    log::info!(
        "Validating {xml_file} with PreFigure schema {}",
        schema.display()
    );

    let schema = schema.to_string_lossy().into_owned();
    let status = if which("xmllint").is_some() {
        std::process::Command::new("xmllint")
            .args(["--noout", "--relaxng", &schema, xml_file])
            .status()
    } else if which("jing").is_some() {
        std::process::Command::new("jing")
            .args([&schema, xml_file])
            .status()
    } else {
        return Err(
            "Validation needs an external RelaxNG validator (xmllint or jing); \
             none was found on PATH."
                .to_string(),
        );
    };

    match status {
        Ok(s) if s.success() => {
            log::info!("{xml_file} is valid");
            Ok(())
        }
        Ok(_) => Err(format!("{xml_file} failed validation")),
        Err(e) => Err(format!("running validator: {e}")),
    }
}

#[cfg(all(feature = "xml-parse", not(target_arch = "wasm32")))]
fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
}

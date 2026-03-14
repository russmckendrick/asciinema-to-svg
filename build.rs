use std::env;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=icons/");

    let icons_dir = Path::new("icons");
    if !icons_dir.exists() {
        return;
    }

    let mut entries: Vec<(String, String)> = Vec::new();

    collect_svgs(icons_dir, &mut entries);

    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("icons.rs");
    let file = fs::File::create(&dest_path).unwrap();
    let mut writer = BufWriter::new(file);

    let mut map = phf_codegen::Map::new();
    for (name, path_data) in &entries {
        map.entry(name.as_str(), &format!("\"{}\"", path_data));
    }

    writeln!(
        &mut writer,
        "static ICONS: phf::Map<&'static str, &'static str> = {};",
        map.build()
    )
    .unwrap();
}

fn collect_svgs(dir: &Path, entries: &mut Vec<(String, String)>) {
    let Ok(read_dir) = fs::read_dir(dir) else {
        return;
    };

    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_svgs(&path, entries);
        } else if path.extension().and_then(|e| e.to_str()) == Some("svg")
            && let Some(name) = path.file_stem().and_then(|s| s.to_str())
            && let Some(path_data) = extract_path_data(&path)
        {
            entries.push((name.to_string(), path_data));
        }
    }
}

fn extract_path_data(svg_path: &Path) -> Option<String> {
    let content = fs::read_to_string(svg_path).ok()?;
    // Find d="..." attribute in <path> element
    let d_start = content.find("d=\"")?;
    let rest = &content[d_start + 3..];
    let d_end = rest.find('"')?;
    let data = &rest[..d_end];
    if data.is_empty() {
        return None;
    }
    Some(data.to_string())
}

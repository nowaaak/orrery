use std::path::{Path, PathBuf};
use std::process::Command;

const RENDER_STAGES: [&str; 8] = [
    "NONE",
    "SKY",
    "SUNSET",
    "CUSTOM_SKY",
    "SUN",
    "MOON",
    "STARS",
    "VOID",
];

fn expand(shaders: &Path, file: &Path, depth: u32) -> Result<String, String> {
    if depth > 16 {
        return Err(format!("include depth exceeded at {}", file.display()));
    }
    let text = std::fs::read_to_string(file).map_err(|err| format!("{}: {err}", file.display()))?;
    let mut out = String::new();
    for line in text.lines() {
        match line.trim().strip_prefix("#include \"/") {
            Some(include) => {
                let target = shaders.join(include.trim_end_matches('"'));
                out.push_str(&expand(shaders, &target, depth + 1)?);
            }
            None => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    Ok(out)
}

fn with_iris_defines(source: &str) -> String {
    let (version, rest) = source.split_once('\n').unwrap_or((source, ""));
    let mut out = format!("{version}\n#define IS_IRIS\n#define MC_VERSION 12101\n");
    for (index, stage) in RENDER_STAGES.iter().enumerate() {
        out.push_str(&format!("#define MC_RENDER_STAGE_{stage} {index}\n"));
    }
    out.push_str(rest);
    out
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let shaders = manifest.join("../../shaderpack/shaders");
    let validator = std::env::var("GLSLANG").unwrap_or_else(|_| "glslangValidator".into());
    let out_dir = std::env::temp_dir().join("orrery-shader-check");
    std::fs::create_dir_all(&out_dir)?;

    let mut programs: Vec<PathBuf> = std::fs::read_dir(&shaders)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("vsh" | "fsh")
            )
        })
        .collect();
    programs.sort();

    let mut failed = Vec::new();
    for program in &programs {
        let name = program
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let stage = if name.ends_with(".vsh") {
            "vert"
        } else {
            "frag"
        };
        let source = with_iris_defines(&expand(&shaders, program, 0)?);
        let target = out_dir.join(format!("{name}.{stage}"));
        std::fs::write(&target, source)?;
        let output = Command::new(&validator).arg(&target).output()?;
        if output.status.success() {
            println!("ok   {name}");
        } else {
            println!("FAIL {name}\n{}", String::from_utf8_lossy(&output.stdout));
            failed.push(name.to_string());
        }
    }
    if failed.is_empty() {
        println!("{} programs compiled", programs.len());
        Ok(())
    } else {
        Err(format!("{} programs failed: {}", failed.len(), failed.join(", ")).into())
    }
}

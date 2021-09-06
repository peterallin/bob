use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use structopt::StructOpt;

fn main() -> Result<()> {
    let options = Options::from_args();
    let presets_filename = options.presets;
    let work_dir = presets_filename.parent().unwrap();
    let work_dir =
        dunce::canonicalize(work_dir).context("Failed to canonicalize working directory")?;
    let presets_json =
        std::fs::read_to_string(&presets_filename).context("Failed to read input file")?;
    let presets: Presets =
        serde_json::from_str(&presets_json).context("Failed to parse input JSON")?;
    let visible_presets: Vec<_> = presets
        .configure_presets
        .iter()
        .filter(|p| !p.hidden && p.binary_dir.is_some())
        .collect();
    let presets = select_presets(&visible_presets);
    for preset in presets {
        config_and_build(&preset.name, &preset.binary_dir.as_ref().unwrap(), &work_dir)?;
    }
    Ok(())
}

#[derive(Deserialize, Debug)]
struct Presets {
    #[serde(rename = "configurePresets")]
    configure_presets: Vec<Preset>,
}

#[derive(Deserialize, Debug)]
struct Preset {
    name: String,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    #[serde(rename = "binaryDir")]
    binary_dir: Option<String>,
    #[serde(default)]
    hidden: bool,
}

#[derive(StructOpt)]
struct Options {
    presets: std::path::PathBuf,
}

fn select_presets<'a>(visible_presets:  &[&'a Preset]) -> impl Iterator<Item=&'a Preset> {
    let names: Vec<_> = visible_presets
        .iter()
        .map(|p| p.display_name.as_ref().unwrap_or_else(|| &p.name).as_str())
        .collect();
    let chosen = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .items(&names)
        .with_prompt("Select preset to build")
        .interact()
        .unwrap();
    let preset = visible_presets[chosen];
    std::iter::once(preset)
}


fn config_and_build(preset: &str, binary_dir: &str, work_dir: &std::path::Path) -> Result<()> {
    cmake(&["--preset", preset], work_dir).context("CMake configure step failed")?;
    cmake(&["--build", binary_dir], work_dir).context("CMake build step failed")?;
    Ok(())
}

fn cmake(args: &[&str], path: &std::path::Path) -> Result<()> {
    let mut child = std::process::Command::new("cmake").current_dir(path).args(args).spawn().context("Failed to spawn CMake")?;
    let exit_status = child.wait().context("Failed executing CMake")?;
    if !exit_status.success() {
        return Err(anyhow!(format!("CMake failed with exit code{}", exit_status)));
    }
    Ok(())
}
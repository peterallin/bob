use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use structopt::StructOpt;

fn main() -> Result<()> {
    let options = Options::from_args();
    let presets_filename = options.presets;
    let work_dir = presets_filename.parent().unwrap();
    let mut preselected_filename = work_dir.to_path_buf();
    preselected_filename.push(".bob");
    let preselected = read_preselected(&preselected_filename).unwrap_or(PreSelected {
        preselected: vec![],
    });
    let work_dir =
        dunce::canonicalize(work_dir).context("Failed to canonicalize working directory")?;
    let presets_json =
        std::fs::read_to_string(&presets_filename).context("Failed to read input file")?;
    let presets: Presets =
        serde_json::from_str(&presets_json).context("Failed to parse input JSON")?;
    let visible_presets: Vec<_> = presets
        .configure_presets
        .into_iter()
        .filter(|p| !p.hidden && p.binary_dir.is_some())
        .collect();
    let presets: Vec<_> = select_presets(&visible_presets, preselected.preselected).collect();
    for preset in presets.iter() {
        println!("----- {} -----", &preset.display_name.as_ref().unwrap());
        config_and_build(
            &preset.name,
            &preset.binary_dir.as_ref().unwrap(),
            &work_dir,
        )?;
    }
    let preset_names : Vec<_> = presets.iter().map(|p| p.name.clone()).collect();
    write_preselected(&preselected_filename, preset_names)?;
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

#[derive(Serialize, Deserialize, Debug)]
struct PreSelected {
    preselected: Vec<String>,
}

#[derive(StructOpt)]
struct Options {
    presets: std::path::PathBuf,
}

fn read_preselected(filename: &std::path::Path) -> Result<PreSelected> {
    let json = std::fs::read_to_string(filename)?;
    Ok(serde_json::from_str(&json)?)
}

fn write_preselected(filename: &std::path::Path, names: Vec<String>) -> Result<()> {
    let json = serde_json::to_string(&PreSelected{preselected: names})?;
    std::fs::write(filename, json)?;
    Ok(())
}

fn select_presets(visible_presets: &[Preset], selected: Vec<String>) -> impl Iterator<Item = &Preset> {
    let items: Vec<_> = visible_presets
        .iter()
        .map(|p| {
            (
                p.display_name.as_ref().unwrap_or_else(|| &p.name).as_str(),
                selected.contains(&p.name),
            )
        })
        .collect();
    let chosen = dialoguer::MultiSelect::new()
        .items_checked(&items)
        .with_prompt("Select presets to build (cursor up/down, space, return)")
        .interact()
        .unwrap();
    visible_presets
        .into_iter()
        .enumerate()
        .filter(move |(i, _)| chosen.contains(i))
        .map(|(_, preset)| preset)
}

fn config_and_build(preset: &str, binary_dir: &str, work_dir: &std::path::Path) -> Result<()> {
    cmake(&["--preset", preset], work_dir).context("CMake configure step failed")?;
    cmake(&["--build", binary_dir], work_dir).context("CMake build step failed")?;
    Ok(())
}

fn cmake(args: &[&str], path: &std::path::Path) -> Result<()> {
    let mut child = std::process::Command::new("cmake")
        .current_dir(path)
        .args(args)
        .spawn()
        .context("Failed to spawn CMake")?;
    let exit_status = child.wait().context("Failed executing CMake")?;
    if !exit_status.success() {
        return Err(anyhow!(format!(
            "CMake failed with exit code{}",
            exit_status
        )));
    }
    Ok(())
}

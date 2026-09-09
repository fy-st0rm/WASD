use crate::config::Direction;
use std::process::Command;

pub fn configure(
  width: u16,
  height: u16,
  direction: &Direction,
) -> Result<(i16, i16, String), Box<dyn std::error::Error>> {
  let virtual_output = find_virtual_output()?;
  let primary_output = find_primary_output()?;

  let mode = format!("{}x{}", width, height);

  let mut command = Command::new("xrandr");

  command
    .arg("--output")
    .arg(&virtual_output)
    .arg("--mode")
    .arg(&mode);

  match direction {
    Direction::Left => {
      command
        .arg("--left-of")
        .arg(&primary_output);
    }

    Direction::Right => {
      command
        .arg("--right-of")
        .arg(&primary_output);
    }

    Direction::Top => {
      command
        .arg("--above")
        .arg(&primary_output);
    }

    Direction::Bottom => {
      command
        .arg("--below")
        .arg(&primary_output);
    }
  }

  let status = command.status()?;

  if !status.success() {
    return Err("xrandr failed".into());
  }

  let geometry = get_geometry(&virtual_output)?;

  println!(
    "Primary display: {}",
    primary_output
  );

  println!(
    "Virtual display: {}x{} at ({}, {})",
    width,
    height,
    geometry.0,
    geometry.1
  );

  Ok((geometry.0, geometry.1, virtual_output))
}

pub fn cleanup() -> Result<(), Box<dyn std::error::Error>> {
  let output = find_virtual_output()?;

  let status = Command::new("xrandr")
    .arg("--output")
    .arg(&output)
    .arg("--off")
    .status()?;

  if !status.success() {
    return Err("Failed to disable virtual display".into());
  }

  println!("Virtual display disabled");

  Ok(())
}

fn find_virtual_output() -> Result<String, Box<dyn std::error::Error>> {
  let output = Command::new("xrandr")
    .arg("--query")
    .output()?;

  let text = String::from_utf8(output.stdout)?;

  for line in text.lines() {
    if line.starts_with("Virtual-") && line.contains(" connected") {
      let name = line
        .split_whitespace()
        .next()
        .ok_or("Invalid xrandr output")?;

      return Ok(name.to_string());
    }
  }

  Err("Could not find virtual output".into())
}

fn find_primary_output() -> Result<String, Box<dyn std::error::Error>> {
  let output = Command::new("xrandr")
    .arg("--query")
    .output()?;

  let text = String::from_utf8(output.stdout)?;

  // Prefer the output explicitly marked as primary.
  for line in text.lines() {
    if line.contains(" connected primary") {
      let name = line
        .split_whitespace()
        .next()
        .ok_or("Invalid xrandr output")?;

      // Don't accidentally select a virtual display.
      if !name.starts_with("Virtual-") {
        return Ok(name.to_string());
      }
    }
  }

  // Fallback: use the first connected non-virtual output.
  for line in text.lines() {
    if !line.contains(" connected") {
      continue;
    }

    let name = line
      .split_whitespace()
      .next()
      .ok_or("Invalid xrandr output")?;

    if !name.starts_with("Virtual-") {
      return Ok(name.to_string());
    }
  }

  Err("Could not find physical display".into())
}

fn get_geometry(
  output_name: &str,
) -> Result<(i16, i16), Box<dyn std::error::Error>> {
  let output = Command::new("xrandr")
    .arg("--query")
    .output()?;

  let text = String::from_utf8(output.stdout)?;

  for line in text.lines() {
    if !line.starts_with(output_name) || !line.contains(" connected") {
      continue;
    }

    for part in line.split_whitespace() {
      // Examples:
      //
      // 1024x768+1366+0
      // 1024x768-1024+0
      // 1024x768+0-768
      //
      // Find the first coordinate sign after WIDTHxHEIGHT.

      let Some(x_pos) = part.find('x') else {
        continue;
      };

      let geometry = &part[x_pos + 1..];

      let Some(first_sign) = geometry.find(['+', '-']) else {
        continue;
      };

      let coords = &geometry[first_sign..];

      let Some(second_sign) = coords[1..].find(['+', '-']) else {
        continue;
      };

      let second_sign = second_sign + 1;

      let x_str = &coords[..second_sign];
      let y_str = &coords[second_sign..];

      if let (Ok(x), Ok(y)) = (
        x_str.parse::<i16>(),
        y_str.parse::<i16>(),
      ) {
        return Ok((x, y));
      }
    }
  }

  Err(
    format!(
      "Could not find geometry for {}",
      output_name
    )
    .into(),
  )
}

pub fn move_workspace(
  workspace: u32,
  output: &str,
) -> Result<(), Box<dyn std::error::Error>> {
  let command = format!(
    "workspace {}; move workspace to output {}",
    workspace,
    output
  );

  let status = Command::new("i3-msg")
    .arg(&command)
    .status()?;

  if !status.success() {
    return Err("Failed to move workspace".into());
  }

  Ok(())
}

use crate::config::Direction;
use std::process::Command;

pub fn configure(
  width: u16,
  height: u16,
  direction: &Direction,
) -> Result<(i16, i16, String), Box<dyn std::error::Error>> {
  let virtual_output = find_virtual_output()?;
  let primary_output = find_primary_output()?;

  let mode = ensure_mode(&virtual_output, width, height)?;

  let mut command = Command::new("xrandr");

  command
    .arg("--output")
    .arg(&virtual_output)
    .arg("--mode")
    .arg(&mode);

  match direction {
    Direction::Left => {
      command.arg("--left-of").arg(&primary_output);
    }

    Direction::Right => {
      command.arg("--right-of").arg(&primary_output);
    }

    Direction::Top => {
      command.arg("--above").arg(&primary_output);
    }

    Direction::Bottom => {
      command.arg("--below").arg(&primary_output);
    }
  }

  let status = command.status()?;

  if !status.success() {
    return Err("xrandr failed".into());
  }

  let geometry = get_geometry(&virtual_output)?;

  println!("Primary display: {}", primary_output);

  println!(
    "Virtual display: {}x{} at ({}, {})",
    width, height, geometry.0, geometry.1
  );

  Ok((geometry.0, geometry.1, virtual_output))
}

fn ensure_mode(
  output: &str,
  width: u16,
  height: u16,
) -> Result<String, Box<dyn std::error::Error>> {
  let requested = format!("{}x{}", width, height);

  let xrandr = Command::new("xrandr").arg("--query").output()?;

  let text = String::from_utf8(xrandr.stdout)?;

  // Check if the requested resolution already exists
  // on the virtual output.
  if let Some(mode) = find_output_mode(&text, output, &requested) {
    return Ok(mode);
  }

  println!("Mode {} not found. Generating modeline...", requested);

  // Generate modeline.
  let cvt = Command::new("cvt")
    .arg(width.to_string())
    .arg(height.to_string())
    .arg("60")
    .output()?;

  if !cvt.status.success() {
    return Err("cvt failed. Is cvt installed?".into());
  }

  let cvt_output = String::from_utf8(cvt.stdout)?;

  let modeline = cvt_output
    .lines()
    .find(|line| line.trim_start().starts_with("Modeline"))
    .ok_or("Could not find Modeline in cvt output")?;

  let mut parts = modeline.split_whitespace();

  parts.next(); // Modeline

  let generated_name = parts
    .next()
    .ok_or("Invalid modeline")?
    .trim_matches('"')
    .to_string();

  let values: Vec<&str> = parts.collect();

  // Check again in case the mode already exists globally.
  let xrandr = Command::new("xrandr").arg("--query").output()?;

  let text = String::from_utf8(xrandr.stdout)?;

  let global_exists = text
    .lines()
    .any(|line| line.split_whitespace().any(|part| part == generated_name));

  if !global_exists {
    println!("Creating XRandR mode: {}", generated_name);

    let mut command = Command::new("xrandr");

    command.arg("--newmode").arg(&generated_name);

    for value in &values {
      command.arg(value);
    }

    let status = command.status()?;

    if !status.success() {
      return Err(format!("Failed to create mode {}", generated_name).into());
    }
  }

  // Add the mode to the virtual output.
  //
  // It may already be attached, so only do this if
  // it isn't already present.
  let xrandr = Command::new("xrandr").arg("--query").output()?;

  let text = String::from_utf8(xrandr.stdout)?;

  if find_output_mode(&text, output, &requested).is_none() {
    let status = Command::new("xrandr")
      .arg("--addmode")
      .arg(output)
      .arg(&generated_name)
      .status()?;

    if !status.success() {
      return Err(format!("Failed to add mode {} to {}", generated_name, output).into());
    }
  }

  println!("Using mode: {}", generated_name);

  Ok(generated_name)
}

fn find_output_mode(xrandr_output: &str, output_name: &str, requested: &str) -> Option<String> {
  let mut found_output = false;

  for line in xrandr_output.lines() {
    // Found our output.
    if line.starts_with(output_name) && line.contains(" connected") {
      found_output = true;
      continue;
    }

    // Another output starts.
    if found_output && !line.starts_with(' ') && !line.starts_with('\t') {
      break;
    }

    if !found_output {
      continue;
    }

    for part in line.split_whitespace() {
      /*
       * Requested:
       *
       * 1080x1920
       *
       * Actual XRandR mode:
       *
       * 1080x1920_60.00
       *
       * Accept either.
       */
      if part == requested || part.starts_with(&format!("{}_", requested)) {
        return Some(part.to_string());
      }
    }
  }

  None
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
  let output = Command::new("xrandr").arg("--query").output()?;

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
  let output = Command::new("xrandr").arg("--query").output()?;

  let text = String::from_utf8(output.stdout)?;

  /*
   * Prefer an explicitly primary display.
   */
  for line in text.lines() {
    if line.contains(" connected primary") {
      let name = line
        .split_whitespace()
        .next()
        .ok_or("Invalid xrandr output")?;

      if !name.starts_with("Virtual-") {
        return Ok(name.to_string());
      }
    }
  }

  /*
   * If no primary display exists, use the first
   * connected non-virtual display.
   */
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

fn get_geometry(output_name: &str) -> Result<(i16, i16), Box<dyn std::error::Error>> {
  let output = Command::new("xrandr").arg("--query").output()?;

  let text = String::from_utf8(output.stdout)?;

  for line in text.lines() {
    if !line.starts_with(output_name) || !line.contains(" connected") {
      continue;
    }

    for part in line.split_whitespace() {
      /*
       * Examples:
       *
       * 1024x768+1366+0
       * 1024x768-1024+0
       * 1024x768+0-768
       */

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

      if let (Ok(x), Ok(y)) = (x_str.parse::<i16>(), y_str.parse::<i16>()) {
        return Ok((x, y));
      }
    }
  }

  Err(format!("Could not find geometry for {}", output_name).into())
}

pub fn move_workspace(workspace: u32, output: &str) -> Result<(), Box<dyn std::error::Error>> {
  let command = format!(
    "workspace {}; move workspace to output {}",
    workspace, output
  );

  let status = Command::new("i3-msg").arg(&command).status()?;

  if !status.success() {
    return Err("Failed to move workspace".into());
  }

  Ok(())
}

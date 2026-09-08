use crate::config::Direction;
use std::process::Command;

pub fn configure(
  width: u16,
  height: u16,
  direction: &Direction,
) -> Result<(i16, i16), Box<dyn std::error::Error>> {
  let output = find_virtual_output()?;

  let mode = format!("{}x{}", width, height);

  let mut command = Command::new("xrandr");

  command
    .arg("--output")
    .arg(&output)
    .arg("--mode")
    .arg(&mode);

  match direction {
    Direction::Left => {
      command.arg("--left-of").arg("eDP-1");
    }
    Direction::Right => {
      command.arg("--right-of").arg("eDP-1");
    }
    Direction::Top => {
      command.arg("--above").arg("eDP-1");
    }
    Direction::Bottom => {
      command.arg("--below").arg("eDP-1");
    }
  }

  let status = command.status()?;

  if !status.success() {
    return Err("xrandr failed".into());
  }

  let geometry = get_geometry(&output)?;

  println!(
    "Virtual display: {}x{} at ({}, {})",
    width, height, geometry.0, geometry.1
  );

  Ok(geometry)
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
    if line.contains(" connected") && line.starts_with("Virtual-") {
      return Ok(line.split_whitespace().next().unwrap().to_string());
    }
  }

  Err("Could not find virtual output".into())
}

fn get_geometry(output_name: &str) -> Result<(i16, i16), Box<dyn std::error::Error>> {
  let output = Command::new("xrandr").arg("--query").output()?;

  let text = String::from_utf8(output.stdout)?;

  for line in text.lines() {
    if !line.starts_with(output_name) || !line.contains(" connected") {
      continue;
    }

    for part in line.split_whitespace() {
      // Find geometry containing the configured position.
      //
      // Examples:
      // 1024x768+1366+0
      // 1024x768-1024+0
      // 1024x768+0-768

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

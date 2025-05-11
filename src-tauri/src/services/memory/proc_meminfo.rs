pub fn get_memtotal_kb() -> std::io::Result<u64> {
  use std::fs;
  use std::io;

  let content = fs::read_to_string("/proc/meminfo")?;
  for line in content.lines() {
    if let Some(mem_kb_str) = line.strip_prefix("MemTotal:") {
      let kb = mem_kb_str
        .trim()
        .split_whitespace()
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "No value found"))?;
      return kb
        .parse::<u64>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e));
    }
  }
  Err(io::Error::new(
    io::ErrorKind::NotFound,
    "MemTotal entry not found",
  ))
}

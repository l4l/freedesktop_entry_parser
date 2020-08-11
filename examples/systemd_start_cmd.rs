use freedesktop_entry_parser::parse_entry;
use std::io::Result;

fn main() -> Result<()> {
    let entry = parse_entry("./test_data/sshd.service")?;
    let start_cmd = entry
        .section("Service")
        .attr("ExecStart")
        .expect("Attribute doesn't exist");
    println!("{}", start_cmd);
    Ok(())
}

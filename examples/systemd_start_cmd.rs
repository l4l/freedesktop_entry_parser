use freedesktop_entry_parser::{Entry, ParseError};

fn main() -> Result<(), ParseError> {
    let entry = Entry::parse_file("./test_data/sshd.service")?;
    let start_cmd = entry
        .section("Service")
        .attr("ExecStart")
        .expect("Attribute doesn't exist");
    println!("{}", start_cmd);
    Ok(())
}

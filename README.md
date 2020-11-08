# Freedesktop Entry Parser

[![crates.io](https://img.shields.io/crates/v/freedesktop_entry_parser.svg)](https://crates.io/crates/freedesktop_entry_parser)
[![docs.rs](https://docs.rs/freedesktop_entry_parser/badge.svg)](https://docs.rs/freedesktop_entry_parser)

A library for parsing FreeDesktop entry files in Rust.
These files are used in the [Desktop Entry](desktop_spec),
[Icon Theme](icon_spec), and [Systemd Unit](systemd) file. They are similar to ini files but are
distinct enough that an ini parse would not work.

[desktop_spec]: https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html
[icon_spec]: https://specifications.freedesktop.org/icon-theme-spec/icon-theme-spec-latest.html
[systemd]: https://www.freedesktop.org/software/systemd/man/systemd.unit.html

## Example Usage

As example input lets use the contents of `sshd.service`
```
[Unit]
Description=OpenSSH Daemon
Wants=sshdgenkeys.service
After=sshdgenkeys.service
After=network.target

[Service]
ExecStart=/usr/bin/sshd -D
ExecReload=/bin/kill -HUP $MAINPID
KillMode=process
Restart=always

[Install]
WantedBy=multi-user.target
```

For example, to print the start command we could do this:
```
use freedesktop_entry_parser::parse_entry;

let entry = parse_entry("./test_data/sshd.service")?;
let start_cmd = entry
    .section("Service")
    .attr("ExecStart")
    .expect("Attribute doesn't exist");
println!("{}", start_cmd);

# Ok::<(), freedesktop_entry_parser::ParseError>(())
```
This prints `/usr/bin/sshd -D`

For more extensive documentaion see [docs.rs](docs) or generate the docs
yourself by cloning the repo and running `cargo doc`.  For more examples
see the [exmaples in the repo](examples).

[docs]: https://docs.rs/freedesktop_entry_parser/0.4.0/freedesktop_entry_parser/
[examples]: https://git.sr.ht/~zethra/freedesktop_entry_parser/tree/master/examples

## Contributing

Please send any and all patches, bugs, and questions to my public inbox
[~zethra/public-inbox@lists.sr.ht](mailto:~zethra/public-inbox@lists.sr.ht)
or submit a ticket to the bug tracker if you feel so inclined
[todo.sr.ht/~zethra/linicon](https://todo.sr.ht/~zethra/linicon).

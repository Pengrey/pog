# pog

pog is a pentesters log tool written in Rust. It is markdown based and works through a files in the file system. It is designed to be used in a terminal and it also provides a TUI interface. It is also designed to be used in a collaborative environment, where multiple users can work on the same log file.

## Usage
To use pog, you can run the following command:

```bash
$ pog --help
Usage: pog [FUNCTION] [OPTIONS]

Functions:
    ingest    Ingest a file into the log
    report    Generate a report from the log
    view      View the log in a TUI interface

Options:
    -h, --help       Print help information
    -V, --version    Print version information
    -v, --verbose    Enable verbose output

$
```

To ingest a file into the log, you can run the following command:

```bash
$ pog ingest --file /path/to/file
```

To generate a report from the log, you can run the following command:

```bash
$ pog report --output /path/to/report.md
```

To view the log in a TUI interface, you can run the following command:

```bash
$ pog view
```

## Contributing
Contributions are welcome! Please open an issue or submit a pull request.

## License
This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details
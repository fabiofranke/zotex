# zotexon
Headless tool to export a Zotero library, using only the web api.

## CLI Arguments
<!-- cli-help-start -->
```console
$ zotexon --help
A command-line tool to export a Zotero library to a file.

Usage: zotexon [OPTIONS] --api-key <API_KEY> --file <FILE>

Options:
      --api-key <API_KEY>    Zotero API Key with read access to your library. Generate a key in your Zotero settings: https://www.zotero.org/settings/keys/new
      --file <FILE>          File that the library will be exported to
      --interval <INTERVAL>  Interval (in seconds) for periodic exports. If not provided, the program will exit after exporting once
      --format <FORMAT>      Format to be used for the export [default: biblatex] [possible values: biblatex, bibtex]
  -h, --help                 Print help
  -V, --version              Print version
```
<!-- cli-help-end -->


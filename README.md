# zotex
Headless tool to export a Zotero library, using only the web api.

## CLI Arguments
<!-- cli-help-start -->
```console
$ zotex --help
A command-line tool to export a Zotero library to a file.

Usage: zotex [OPTIONS] --api-key <API_KEY> --file <FILE>

Options:
  -a, --api-key <API_KEY>    Zotero API Key with read access to your library. Generate a key in your Zotero settings: https://www.zotero.org/settings/keys/new
  -f, --file <FILE>          File that the library will be exported to
  -i, --interval <INTERVAL>  Interval (in seconds) for periodic exports. If not provided, the program will exit after exporting once
  -h, --help                 Print help
  -V, --version              Print version
```
<!-- cli-help-end -->


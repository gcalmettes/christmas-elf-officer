# christmas-elf-officer
An Advent of Code cheerful friend

<img style="display: block; margin-left: auto; margin-right: auto; width: 95%;" src="https://raw.githubusercontent.com/gcalmettes/christmas-elf-officer/main/img/hld-ceo.png"></img>

# Configuration

## Environment variables:

Runtime configuration is set via environment variables, see `src/config.rs`. Implemented via figment and once_cell.
Any configuration settings will be locally loaded if a `.env.local.yaml` file is present.

## Command line configuration:

CLI arguments will override any configuration setting set through local file or env var.

* `--all-years`: whether to also retrieve the private leaderboard for the past AOC events.



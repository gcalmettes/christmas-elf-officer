# christmas-elf-officer
An Advent of Code cheerful friend

# Configuration
Runtime configuration is set via environment variables, see `src/config.rs`. Implemented via figment and once_cell.
Any configuration settings will be locally loaded if a `.env.local.yaml` file is present.

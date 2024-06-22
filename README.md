# christmas-elf-officer
An Advent of Code cheerful friend

# Architecture

<img style="display: block; margin-left: auto; margin-right: auto; width: 95%;" src="https://raw.githubusercontent.com/gcalmettes/christmas-elf-officer/main/img/hld-ceo.png"></img>

# Configuration

## Settings

Configuration of the bot can be achieve via (from lower to higher precedence):
* local configuration file (`yaml`),
* environment variables,
* or command line flags (limited to some settings only).


| Setting                                   | Mandatory | Description                                                                                                            | default (if any)          |
|-------------------------------------------|-----------|------------------------------------------------------------------------------------------------------------------------|---------------------------|
| `trace_level`                             |           | trace level for bot logs (on server)                                                                                   |`INFO`                     |
| `slack_token`                             | ✅        | [Bot token](https://api.slack.com/authentication/token-types#bot) associated with your slack app. Starts with `xoxb-`  |                           |
| `slack_app_token`                         | ✅        | [App level token](https://api.slack.com/authentication/token-types#app-level) for your workspace. Starts with `xapp-`  |                           |
| `slack_default_channel`                   | ✅        | the slack channel ID to receive the AOC event updates                                                                  |                           |
| `slack_monitoring_channel`                |           | the slack channel ID to reveive heartbeats and monitoring events                                                       | `None`                    |
| `slack_bots_authorized_ids`               |           | list of slack bot ID for the bot to ignore messages from                                                               | `None``                   |
| `global_leaderboard_polling_interval_sec` |           | polling interval (in seconds) to refresh updates from the GLOBAL leaderboard                                           | 300                       |
| `aoc_base_url`                            |           | base url to check AOC updates from (e.g.: can be changed for local development purpose)                                |`https://adventofcode.com` |
| `aoc_api_timeout`                         |           | timeout (in seconds) on requests made to AOC server                                                                    | 5                         |
| `aoc_private_leaderboard_id`              | ✅        | private leaderboard ID from which the bot will compute its metrics and updates                                         |                           |
| `aoc_session_cookie`                      | ✅        | AOC session cookie so the bot can access the private leaderboard specified                                             |                           |
| `all_years`                               |           | whether to load all the previous AOC years or not in the bot internal database                                         |`false`                    |

### Local `yaml` configuration file

Any configuration settings will be locally loaded if a `.env.local.yaml` file is present.
An exemple of configuration file can be found below:

```
slack_token: xoxb-XXXXXXXXXXXXX-XXXXXXXXXXXXX-XXXXXXXXXXXXXXXXXXXXXXXX
slack_app_token: xapp-1-XXXXXXXXXXX-XXXXXXXXXXXXX-XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
slack_default_channel: C000X9X0XX
slack_monitoring_channel: C111X9X1XX
slack_bots_authorized_ids:
  - B000XX0X0X0
aoc_private_leaderboard_id: 000000
aoc_session_cookie: xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
all_years: true
```

### Environment variables

For each specific setting, the corresponding environment variable name to override it is the setting's name in uppercase. For example, setting the environment variable
`SLACK_TOKEN="xxx123"` would set the `slack_token` local settings to the `xxx123` value.

### Command line flags

CLI arguments will override any configuration setting set through local file or env var.

* `--all-years`: whether to also retrieve the private leaderboard for the past AOC events.


## Create slack app for the bot

The bot interacts with the slack API and requires some specific permissions to be able to reads and posts to the channels
it has been invited to.
The manifest below can be used as a basis to create the slack app that will be used by the bot.

```
display_information:
  name: <Name of the slack application>
  description: <Description of the slack application>
  background_color: "#040b1c"
features:
  app_home:
    home_tab_enabled: true
    messages_tab_enabled: false
    messages_tab_read_only_enabled: false
  bot_user:
    display_name: <Name of the slack bot user to be used>
    always_online: false
oauth_config:
  scopes:
    bot:
      - channels:join
      - channels:read
      - groups:read
      - chat:write
      - chat:write.customize
      - channels:history
      - incoming-webhook
settings:
  event_subscriptions:
    bot_events:
      - app_home_opened
      - message.channels
  interactivity:
    is_enabled: true
  org_deploy_enabled: false
  socket_mode_enabled: true
  token_rotation_enabled: false
```

Go to the [Slack API app creation page](https://api.slack.com/apps?new_app=1) and initialize a new
application using the above manifest.

<img style="display: block; margin-left: auto; margin-right: auto; width: 60%;" src="https://raw.githubusercontent.com/gcalmettes/christmas-elf-officer/main/img/slack-app-from-manifest.png"></img>

Once created, retrieve the Bot User OAuth Token (`slack_token`) in the `Oauth & Permissions` section.

The App Level Token (`slack_app_token`) can be retrieved from the `Basic Information` > `App Level Token` section.

# Build & Deploy the bot

Directly build the binary (through `cargo build --release`) or use the provided `Dockerfile` to package the bot in a docker container.

```
docker build -t ceo:1.0.0 .
```

Start the bot, passing any env variables or mounting the configuration file:

```
docker run --name ceo-bot -d --rm --env-file path-to-your.env ceo:1.0.0
```

an example env file can be found below:

```
SLACK_TOKEN=xoxb-0000000000000-0000000000000-000000000000000000000000
SLACK_APP_TOKEN=xapp-1-00000000000-0000000000000-0000000000000000000000000000000000000000000000000000000000000000
SLACK_DEFAULT_CHANNEL=C000000000
SLACK_MONITORING_CHANNEL=C000000000
SLACK_BOTS_AUTHORIZED_IDS=[B011111111]
AOC_BASE_URL=https://adventofcode.com
AOC_PRIVATE_LEADERBOARD_ID=00000000
AOC_SESSION_COOKIE=xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
ALL_YEARS=true
```

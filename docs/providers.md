# Discoverable monitor sources

ESP Gauge keeps integrations behind small sampling adapters. Gauges store source IDs and display ranges on the PCB; account tokens, local paths and provider credentials never become board configuration.

| Provider | Discovery and readings | Unavailable cases |
| --- | --- | --- |
| Codex | CLI `app-server` → `account/rateLimits/read` and `account/usage/read`; local turn history and held writer locks for working agents | API-key accounts may have no subscription quotas. The returned window duration determines five-hour/weekly labels; absent windows are omitted. Old CLI versions or changed local schemas can lack some readings. |
| Claude Code | Native CLI discovery; existing credentials file or macOS Keychain; Anthropic OAuth usage endpoint | Expired or absent login, API-key-only accounts, endpoint changes or denied keychain access. The app explains login failures and never attempts credential refresh. |
| OpenCode | Native CLI/process discovery; read-only local SQLite messages for daily output tokens and estimated USD cost | Missing or incompatible local database. API providers have different quota models, so no universal five-hour/weekly limit is assumed. |
| codeslop / T3 Code | Runtime file and live server process, verified through the loopback environment descriptor; read-only projection database | Server stopped, inaccessible environment descriptor or incompatible database. No remote server or external account is paired automatically. |
| Super Tracker | Production `https://supertracker.nz/api/v1/index/latest`, every five minutes only while used by a gauge | Offline, production DNS not yet live, malformed response or data more than three UTC days old. No silent development-domain fallback. |

CLI process counts include idle sessions and exclude background app servers. Working Codex agents require both an unfinished turn and an actively held writer lock, so abandoned history after a crash is not counted. codeslop/T3 working agents are running/starting sessions in unarchived, undeleted tasks. Context-window usage uses the largest latest reading among ready/running/starting sessions. Usage and history readings contain counts only; prompts, conversations and file contents are not exported.

Local daily totals use the computer's midnight, except Codex account token buckets, which use the account API's UTC dates. Quota readings mean percentage **used**. Reset times are shown in the selected source's description when reported by the provider. Missing values rest host-driven gauges at zero electrical output even if the calibrated lower endpoint is above zero.

Discovery respects `CODEX_HOME`, `XDG_DATA_HOME`, `T3_HOME` and `CODESLOP_HOME`; standard home-directory installations work without configuration. Executable discovery also searches common npm, Homebrew, fnm/nvm and native CLI installation locations, because tray apps often inherit a smaller PATH than interactive terminals.

Sources: [Codex app-server](https://learn.chatgpt.com/docs/app-server), [Claude Code monitoring](https://code.claude.com/docs/en/monitoring-usage), [OpenCode server](https://opencode.ai/docs/server/), [Super Tracker](https://supertracker.nz). Claude's subscription usage adapter uses the endpoint consumed by its CLI; it is not a separately guaranteed public API.

Super Tracker data is attributed to Super Tracker and its source retailers under CC BY 4.0. It is a daily nowcast, not an official statistic. Its headline base is 1,000; the default gauge range is 900–1,100. The integration was checked against the development API schema, and releases always use the requested production domain.

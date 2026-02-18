# Run Contracts

Sempal emits machine-readable run artifacts for agents and automation.

## Artifact path

At startup, the app creates/append to:

- `<log_path>/run_contract_<run_id>.ndjson`

`log_path` is the app logs directory resolved by `app_dirs` (same location used by regular logs).

## Event schema

Each line in the NDJSON file is a JSON object with:

- `run_id` (string): Stable ID for the process run.
- `git_sha` (string): Short Git SHA if available, otherwise `<unknown>`.
- `cfg_path` (string): App root config directory path.
- `log_path` (string): Log directory path.
- `startup_phase` (string): One of `startup`, `runtime`, `shutdown`.
- `exit_status` (string): Phase status (`running`, `success`, `error`, or implementation-specific values).
- `timestamp_utc` (string): Unix epoch seconds when the event was recorded.
- `process_id` (number): OS process ID.

## Example

```json
{"run_id":"1708260012345678900-1234","git_sha":"a1b2c3d","cfg_path":"/tmp/.sempal","log_path":"/tmp/.sempal/logs","startup_phase":"startup","exit_status":"running","timestamp_utc":"1708260012","process_id":1234}
```

## Deterministic assertions

Agents should assert:

- At least one `startup` event exists for a new run.
- A `shutdown` event exists with `exit_status` either `success` or `error`.
- The run artifact path uses the matching `run_id` for the current run.

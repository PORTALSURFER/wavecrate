# Run Contracts

Sempal emits machine-readable run artifacts for agents and automation.

## Artifact path

At startup, the app creates/append to:

- `<log_path>/contracts/run_contract_<run_id>.ndjson`

It also writes:

- `<log_path>/contracts/run_manifest_<run_id>.json`

`log_path` is the app logs directory resolved by `app_dirs` (same location used by regular logs).

## Event schema

Each line in the NDJSON file is a JSON object with:

- `run_id` (string): Stable ID for the process run.
- `git_sha` (string): Short Git SHA if available, otherwise `<unknown>`.
- `cfg_path` (string): App root config directory path.
- `log_path` (string): Log directory path.
- `startup_phase` (string): Coarse phase (`startup`, `runtime`, `shutdown`).
- `milestone` (string): Structured milestone (`startup_begin`, `runtime_started`, `runtime_exit`).
- `exit_status` (string): Milestone status (`running`, `success`, `error`).
- `timestamp_utc` (string): Unix epoch seconds when the event was recorded.
- `process_id` (number): OS process ID.
- `manifest_path` (string): Absolute path to the companion run manifest JSON file.
- `artifact_path` (string): Absolute path to this artifact file.

## Manifest schema

`run_manifest_<run_id>.json` is a compact machine-readable run summary with:

- `run_id` (string): Stable ID for the process run.
- `git_sha` (string): Git SHA used for reproducibility.
- `cfg_path` (string): App config root path.
- `log_path` (string): App log directory path.
- `process_id` (number): OS process ID.
- `executable_path` (string): Absolute executable path used for the run.
- `working_directory` (string): Working directory for process execution.
- `arg_count` (number): Argument count observed at launch.
- `debug` (boolean): Whether the run used debug assertions.
- `started_utc` (string): Process contract start timestamp.
- `completed_utc` (string): Process contract completion timestamp.
- `exit_status` (string): Final process exit status (`success` or `error`).
- `artifact_path` (string): Absolute path to the NDJSON artifact file.
- `manifest_path` (string): Absolute path to this manifest file.
- `milestones` (array): Ordered array of recorded milestones with `name`, `startup_phase`, `status`, `timestamp_utc`.

## Example

```json
{"run_id":"1708260012345678900-1234","git_sha":"a1b2c3d","cfg_path":"/tmp/.sempal","log_path":"/tmp/.sempal/logs/contracts","startup_phase":"startup","milestone":"startup_begin","exit_status":"running","timestamp_utc":"1708260012","process_id":1234,"manifest_path":"/tmp/.sempal/logs/contracts/run_manifest_1708260012345678900-1234.json","artifact_path":"/tmp/.sempal/logs/contracts/run_contract_1708260012345678900-1234.ndjson"}
```

```json
{
  "run_id": "1708260012345678900-1234",
  "git_sha": "a1b2c3d",
  "cfg_path": "/tmp/.sempal",
  "log_path": "/tmp/.sempal/logs/contracts",
  "process_id": 1234,
  "executable_path": "/usr/local/bin/sempal",
  "working_directory": "/tmp/agent-workdir",
  "arg_count": 1,
  "debug": false,
  "started_utc": "1708260012",
  "completed_utc": "1708260019",
  "exit_status": "success",
  "artifact_path": "/tmp/.sempal/logs/contracts/run_contract_1708260012345678900-1234.ndjson",
  "manifest_path": "/tmp/.sempal/logs/contracts/run_manifest_1708260012345678900-1234.json",
  "milestones": [
    {
      "name": "startup_begin",
      "startup_phase": "startup",
      "status": "running",
      "timestamp_utc": "1708260012"
    },
    {
      "name": "runtime_started",
      "startup_phase": "runtime",
      "status": "running",
      "timestamp_utc": "1708260013"
    },
    {
      "name": "runtime_exit",
      "startup_phase": "shutdown",
      "status": "success",
      "timestamp_utc": "1708260019"
    }
  ]
}
```

## Deterministic assertions

Agents should assert:

- Every event has a matching `run_id` and one of: `startup_begin`, `runtime_started`, `runtime_exit`, `startup_failed`.
- `startup_begin -> runtime_started -> runtime_exit` appears in order for normal runs.
- `startup_begin -> startup_failed` appears in order for startup-fail runs, with final `exit_status=error`.
- Manifest `artifact_path` matches the NDJSON artifact path.
- Manifest `milestones` contains the same milestones in the same order.
- Final event `exit_status` equals manifest `exit_status` (`success` or `error`).

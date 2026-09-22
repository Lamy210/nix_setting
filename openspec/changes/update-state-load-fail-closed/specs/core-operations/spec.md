## ADDED Requirements

### Requirement: State 読み込みの fail-closed semantics

core SHALL distinguish an intentionally missing State file from an existing State file that cannot be trusted. Missing state is a valid uninitialized condition, while read or parse failure of an existing state MUST be surfaced as an error and MUST NOT be converted to an empty/default State.

#### Scenario: State file が存在しない

- **WHEN** `StateStore::load` is called and the state file does not exist
- **THEN** it returns a successful missing-state result
- **AND** callers may apply their documented first-run defaults

#### Scenario: valid legacy State を読み込む

- **WHEN** an existing state JSON is valid but omits newer optional fields such as `source` or `profile`
- **THEN** it is loaded successfully
- **AND** omitted compatible fields retain their existing default/None semantics

#### Scenario: malformed State JSON

- **WHEN** the state file exists but contains malformed or incompatible JSON
- **THEN** `StateStore::load` returns a structured error
- **AND** callers MUST NOT treat the state as missing or replace it with `State::default()`

#### Scenario: State file の read failure

- **WHEN** the state path exists but cannot be read as a state file
- **THEN** `StateStore::load` returns a structured error
- **AND** the error is distinguishable from a missing file

### Requirement: State-dependent operation の fail-fast

Operations that depend on persisted source/profile/state semantics SHALL load and validate existing State before performing effects that could mutate the machine, checkout, or persisted State. A state read failure MUST abort the operation rather than silently selecting checkout/default/stable behavior.

#### Scenario: corrupt State で mutation を開始しない

- **WHEN** an existing state file is unreadable or malformed
- **AND** apply, rollback, update, source initialization, or profile mutation is requested
- **THEN** the operation returns an error before its state-dependent external mutation begins
- **AND** the existing state file is not overwritten with a default State

#### Scenario: managed source を checkout へ fallback しない

- **WHEN** an existing state file cannot be loaded
- **AND** source resolution or managed repo-file loading is requested
- **THEN** core returns a state error
- **AND** it MUST NOT infer Local/Git checkout behavior from the missing in-memory State

#### Scenario: corrupt State で stable channel を選ばない

- **WHEN** an existing state file cannot be loaded
- **AND** self-update or Dashboard release resolution needs the selected channel
- **THEN** the caller receives an explicit state error
- **AND** `stable` is not selected merely because State loading failed

#### Scenario: read-only surface が corruption を未設定表示しない

- **WHEN** status, diagnostics, source status, or Dashboard observes an existing invalid state file
- **THEN** the surface reports the state read failure explicitly
- **AND** it MUST NOT report the state as simply uninitialized or absent

# Requirement 2: Delete `crates/cloud_object_models/`

## Verdict: Cannot Remove

## Reason summary

`cloud_object_models` is the canonical definition site for nearly every domain type used by the Warp cloud sync system. It defines:

- The `ObjectClient` trait (the async interface for all cloud object CRUD operations)
- All concrete model structs (workflows, notebooks, folders, environments, MCP servers, preferences, AI facts, execution profiles, scheduled agents, agent configs, env vars, workflow enums)
- Type aliases (`CloudXxx`, `ServerXxx`) that combine generic cloud-object wrappers with those models
- Permission enums (`ActionPermission`, `ComputerUsePermission`, `WriteToPtyPermission`, etc.)
- Command execution predicates and default allowlists/denylists
- The `JsonModel` trait and `JsonSerializer` for generic-string-object serialization
- Re-exports from `mcp_types` (transport types)

The crate is imported by **app/Cargo.toml** as a workspace dependency and consumed in 40+ source files across AI, cloud object, drive, settings, workflows, notebooks, search, and infrastructure modules.

## Types exported and their consumers

### ObjectClient trait + MockObjectClient
- Defined in: `crates/cloud_object_models/src/client_types.rs`
- Trait implemented by `ServerApi` in: `app/src/infra/server_api/object.rs`
- `MockObjectClient` (auto-generated mock via mockall) used in test files:
  - `app/src/cloud_object/model/model_tests.rs`
  - `app/src/pane_group/pane/view/header/mod_tests.rs`
  - `app/src/search/ai_context_menu/notebooks/data_source_tests.rs`
  - `app/src/search/ai_context_menu/rules/data_source_tests.rs`
  - `app/src/search/command_palette/data_sources_tests.rs`
  - `app/src/settings/cloud_preferences_syncer_tests.rs`

### AgentConfig / CloudAgentConfig / CloudAgentConfigModel
- Defined in: `crates/cloud_object_models/src/cloud_agent_config.rs`
- Used by: `app/src/ai/cloud_agent_config/mod.rs`

### AIExecutionProfile / ActionPermission / ComputerUsePermission / WriteToPtyPermission / RunAgentsPermission / AskUserQuestionPermission
- Defined in: `crates/cloud_object_models/src/ai_execution_profile.rs`
- Used by:
  - `app/src/ai/execution_profiles/mod.rs` (re-exports all profile types)
  - `app/src/settings/ai.rs` (re-exports `AgentModeCommandExecutionPredicate`, allowlist/denylist)
  - `app/src/workspaces/workspace.rs` (imports `ActionPermission`, `ComputerUsePermission`, `AgentModeCommandExecutionPredicate`)

### ScheduledAmbientAgent / AgentConfigSnapshot / HarnessConfig / HarnessModelConfig / HarnessAuthSecretsConfig
- Defined in: `crates/cloud_object_models/src/scheduled_ambient_agent.rs`
- Used by:
  - `app/src/ai/ambient_agents/scheduled.rs` (re-exports `CloudScheduledAmbientAgent`, `ScheduledAmbientAgent`)
  - `app/src/ai/ambient_agents/task.rs` (imports `HarnessModelConfig`, `AgentConfigSnapshot`, `HarnessAuthSecretsConfig`, `HarnessConfig`)

### AmbientAgentEnvironment / GithubRepo / BaseImage / GcpProviderConfig / AwsProviderConfig / ProvidersConfig
- Defined in: `crates/cloud_object_models/src/cloud_environment.rs`
- Used by: `app/src/ai/cloud_environments/mod.rs`

### AIFact / AIMemory / CloudAIFact / CloudAIFactModel / SuggestedLoggingId
- Defined in: `crates/cloud_object_models/src/ai_fact.rs`
- Used by:
  - `app/src/ai/facts/mod.rs`
  - `app/src/ai/agent/mod.rs` (re-exports `SuggestedLoggingId`)

### MCPServer / CloudMCPServer / TemplatableMCPServer / TransportType / CLIServer / ServerSentEvents
- Defined in: `crates/cloud_object_models/src/mcp.rs` (plus re-exports from `mcp_types`)
- Used by:
  - `app/src/ai/mcp/mod.rs` (re-exports transport types and server types)
  - `app/src/ai/mcp/templatable.rs` (re-exports templatable types)

### CloudFolder / CloudFolderModel / ServerFolder
- Defined in: `crates/cloud_object_models/src/folder.rs`
- Used by:
  - `app/src/cloud_object/breadcrumbs.rs`
  - `app/src/cloud_object/mod.rs`
  - `app/src/cloud_object/model/persistence.rs`
  - `app/src/cloud_object/model/view.rs`
  - `app/src/drive/folders/mod.rs`
  - `app/src/drive/items.rs`
  - `app/src/persistence/mod.rs`
  - `app/src/search/command_palette/warp_drive/data_source.rs`

### Workflow / CloudWorkflow / CloudWorkflowModel / WorkflowId / Argument / ArgumentType
- Defined in: `crates/cloud_object_models/src/workflow.rs`
- Used by:
  - `app/src/workflows/mod.rs` (re-exports `CloudWorkflow`, `CloudWorkflowModel`, `WorkflowId`)
  - `app/src/workflows/workflow.rs` (re-exports `Argument`, `ArgumentType`, `Workflow`)
  - `app/src/infra/server_api/object.rs` (uses `WorkflowId`)

### WorkflowEnum / EnumVariants / CloudWorkflowEnum
- Defined in: `crates/cloud_object_models/src/workflow_enum.rs`
- Used by: `app/src/workflows/workflow_enum.rs`

### CloudNotebook / CloudNotebookModel / NotebookId / SerializedNotebook
- Defined in: `crates/cloud_object_models/src/notebook.rs`
- Used by: `app/src/notebooks/mod.rs`

### Preference / CloudPreference / CloudPreferenceModel / Platform
- Defined in: `crates/cloud_object_models/src/preference.rs`
- Used by:
  - `app/src/settings/cloud_preferences.rs`
  - `app/src/settings/cloud_preferences_syncer.rs` (uses `JsonSerializer`)

### EnvVarCollection / EnvVar / EnvVarValue / EnvVarSecretCommand / ExternalSecret / OnePasswordSecret / LastPassSecret
- Defined in: `crates/cloud_object_models/src/env_vars.rs`
- Used by:
  - `app/src/env_vars/mod.rs`
  - `app/src/env_vars/view/command_dialog/mod.rs` (uses `EnvVarSecretCommand`)
  - `app/src/external_secrets/mod.rs` (uses `ExternalSecret`, `LastPassSecret`, `OnePasswordSecret`)

### JsonModel trait / JsonSerializer
- Defined in: `crates/cloud_object_models/src/json_model.rs`
- Used by:
  - `app/src/infra/server_api/object.rs` (uses `JsonSerializer`)
  - `app/src/settings/cloud_preferences_syncer.rs` (uses `JsonSerializer`)
  - `app/src/cloud_object/model/generic_string_model.rs` (uses `ObjectClient`)

### ServerCloudObject (enum dispatching all server object variants)
- Defined in: `crates/cloud_object_models/src/server_cloud_object.rs`
- Used by: `app/src/infra/server_api/object.rs`, `app/src/cloud_object/mod.rs`

### UserProfileWithUID / UserProfileIdAndName / TeamProfileIdAndName
- Defined in: `crates/cloud_object_models/src/user_profile.rs`
- Used indirectly via `ServerCloudObject` and `ObjectClient` trait

## Complete file listing

| File path | Imports from `cloud_object_models` | Role |
|---|---|---|
| `app/src/ai/agent/mod.rs` | `SuggestedLoggingId` | Re-exports logging ID for agent suggestions |
| `app/src/ai/ambient_agents/scheduled.rs` | `CloudScheduledAmbientAgent`, `CloudScheduledAmbientAgentModel`, `ScheduledAmbientAgent` | Scheduled agent management |
| `app/src/ai/ambient_agents/task.rs` | `HarnessModelConfig`, `AgentConfigSnapshot`, `HarnessAuthSecretsConfig`, `HarnessConfig` | Agent task execution config |
| `app/src/ai/cloud_agent_config/mod.rs` | `AgentConfig`, `CloudAgentConfig`, `CloudAgentConfigModel` | Cloud agent config management |
| `app/src/ai/cloud_environments/mod.rs` | `AmbientAgentEnvironment`, `AwsProviderConfig`, `BaseImage`, `CloudAmbientAgentEnvironment`, `CloudAmbientAgentEnvironmentModel`, `GcpProviderConfig`, `GithubRepo`, `ProvidersConfig` | Environment definitions |
| `app/src/ai/execution_profiles/mod.rs` | `AIExecutionProfile`, `ActionPermission`, `AskUserQuestionPermission`, `CloudAIExecutionProfile`, `CloudAIExecutionProfileModel`, `ComputerUsePermission`, `RunAgentsPermission`, `WriteToPtyPermission`, `PROFILE_NAME_MAX_LENGTH` | Execution profile management |
| `app/src/ai/facts/mod.rs` | `AIFact`, `AIMemory`, `CloudAIFact`, `CloudAIFactModel` | AI facts/memories |
| `app/src/ai/mcp/mod.rs` | `CLIServer`, `JSONMCPServer`, `JSONTransportType`, `ServerSentEvents`, `StaticEnvVar`, `StaticHeader`, `CloudMCPServer`, `CloudMCPServerModel`, `MCPServer`, `MCPServerState`, `TransportType` | MCP server management |
| `app/src/ai/mcp/templatable.rs` | `CloudTemplatableMCPServer`, `CloudTemplatableMCPServerModel`, `GalleryData`, `JsonTemplate`, `TemplatableMCPServer`, `TemplateVariable` | Templatable MCP servers |
| `app/src/cloud_object/breadcrumbs.rs` | `CloudFolder` | Breadcrumb navigation |
| `app/src/cloud_object/mod.rs` | `ObjectClient`, `ServerCloudObject`, various cloud object types | Core cloud object module |
| `app/src/cloud_object/model/actions.rs` | Action-related types | Object actions |
| `app/src/cloud_object/model/generic_string_model.rs` | `ObjectClient` | Generic string object model |
| `app/src/cloud_object/model/model_tests.rs` | `MockObjectClient`, `ObjectClient` | Tests |
| `app/src/cloud_object/model/persistence.rs` | `CloudFolderModel`, `CloudFolder` | Persistence layer |
| `app/src/cloud_object/model/view.rs` | `CloudFolder` | View layer |
| `app/src/drive/folders/mod.rs` | `CloudFolder`, `CloudFolderModel`, `ObjectClient` | Drive folders |
| `app/src/drive/items.rs` | `CloudFolder` | Drive items |
| `app/src/env_vars/mod.rs` | `CloudEnvVarCollection`, `CloudEnvVarCollectionModel`, `EnvVar`, `EnvVarCollection`, `EnvVarValue` | Env var management |
| `app/src/env_vars/view/command_dialog/mod.rs` | `EnvVarSecretCommand` | Env var UI |
| `app/src/external_secrets/mod.rs` | `ExternalSecret`, `LastPassSecret`, `OnePasswordSecret` | External secrets |
| `app/src/infra/server_api/object.rs` | `GetCloudObjectResponse`, `InitialLoadResponse`, `ObjectActionHistory`, `ObjectActionType`, `ObjectDeleteResult`, `ObjectMetadataUpdateResult`, `ObjectPermissionUpdateResult`, `ObjectPermissionsUpdateData`, `ObjectUpdateMessage`, `GuestIdentifier`, `ObjectClient`, `JsonSerializer`, `WorkflowId` | Server API implementation |
| `app/src/notebooks/mod.rs` | `CloudNotebook`, `CloudNotebookModel`, `NotebookId`, `SerializedNotebook`, `ObjectClient` | Notebook management |
| `app/src/pane_group/pane/view/header/mod_tests.rs` | `MockObjectClient` | Tests |
| `app/src/persistence/mod.rs` | `CloudFolder` | Local persistence |
| `app/src/search/ai_context_menu/notebooks/data_source_tests.rs` | `MockObjectClient` | Tests |
| `app/src/search/ai_context_menu/rules/data_source_tests.rs` | `MockObjectClient` | Tests |
| `app/src/search/command_palette/data_sources_tests.rs` | `MockObjectClient` | Tests |
| `app/src/search/command_palette/warp_drive/data_source.rs` | `CloudFolder` | Search data source |
| `app/src/settings/ai.rs` | `AgentModeCommandExecutionPredicate`, `DEFAULT_COMMAND_EXECUTION_ALLOWLIST`, `DEFAULT_COMMAND_EXECUTION_DENYLIST` | AI settings |
| `app/src/settings/cloud_preferences.rs` | `CloudPreference`, `CloudPreferenceModel`, `Platform`, `Preference` | Cloud preferences |
| `app/src/settings/cloud_preferences_syncer.rs` | `JsonSerializer` | Preference syncing |
| `app/src/settings/cloud_preferences_syncer_tests.rs` | `MockObjectClient` | Tests |
| `app/src/workflows/mod.rs` | `CloudWorkflow`, `CloudWorkflowModel`, `WorkflowId`, `ObjectClient` | Workflow management |
| `app/src/workflows/workflow.rs` | `Argument`, `ArgumentType`, `Workflow` | Workflow types |
| `app/src/workflows/workflow_enum.rs` | `CloudWorkflowEnum`, `CloudWorkflowEnumModel`, `EnumVariants`, `WorkflowEnum` | Workflow enums |
| `app/src/workspaces/workspace.rs` | `ActionPermission`, `ComputerUsePermission`, `AgentModeCommandExecutionPredicate` | Workspace permissions |

## Dependency chain

```
cloud_object_models
├── depends on: ai, cloud_objects, mcp_types, settings, warp_cli, warp_core, warp_util, warp_types
│                session-sharing-protocol, warp-workflows, handlebars, schemars, regex, ...
├── depended on by: app (Cargo.toml workspace dep)
└── feature: "agent_mode_evals" (conditional eval profile), "test-util" (re-exports cloud_objects test-util)
```

The crate sits at a critical junction: it combines the generic cloud-object machinery from `cloud_objects` with domain-specific Warp types (from `ai`, `warp_cli`, `settings`, etc.) to produce the concrete typed models consumed throughout the app.

## What would need to happen to eventually remove it

1. **Inline models into their consuming modules.** Each model struct and its type aliases could be moved into the app module that owns that domain (e.g., `AIExecutionProfile` into `app/src/ai/execution_profiles/`, `Workflow` into `app/src/workflows/`, etc.). This is a large refactor touching 40+ files.

2. **Move `ObjectClient` trait into `app/src/infra/` or a new thin crate.** The trait is the key abstraction used for dependency injection in tests (via `MockObjectClient`). It could live in a purpose-built interface crate or be inlined.

3. **Move `JsonModel` and `JsonSerializer` into `cloud_objects` or a utility crate.** These are generic serialization helpers used by all JSON-backed models.

4. **Move `ServerCloudObject` enum into the app.** It depends on every concrete model type, so it would naturally land alongside the consumer code.

5. **Move permission types (`ActionPermission`, `ComputerUsePermission`, etc.) into a shared settings/permissions module.** These are used across AI execution profiles, workspaces, and settings.

6. **Move `AgentModeCommandExecutionPredicate` and default lists into settings.** Already re-exported into `app/src/settings/ai.rs`.

7. **Re-export MCP transport types directly from `mcp_types`.** The crate already just re-exports them.

**Estimated effort:** Large (2-3 days). The crate is a pure "model definition" crate with no business logic beyond serialization — it is conceptually a horizontal layer, not dead code. Removing it is a refactoring choice (inlining definitions), not dead-code deletion.


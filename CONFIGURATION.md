AKD-watch uses a layered configuration system that supports multiple sources in order of increasing priority:

1. **Configuration file**: `config.toml`, `config.yaml`, or `config.json`
2. **Environment variables** (with prefix `AKD_WATCH__`): e.g., `AKD_WATCH__SLEEP_SECONDS=30`

### Configuration File

See `config.example.toml` for a complete example configuration file.

#### Root Configuration Options

- `bind_address`: Address to bind the web server to (defaults to `3000`, web crate only)
- `sleep_seconds` (optional): Time to wait between audit cycles in seconds (defaults to 30, auditor crate only)
- `data_directory`: Directory to store data files for file-based storage backends
- `namespaces`: Array of namespace configurations to audit (auditor crate only)
- `signing`: Signing key configuration
- `signature_storage`: Storage backend configuration
- `namespace_storage`: Namespace state storage configuration

#### Namespace State Storage Configuration

The namespace state (e.g., last verified epochs and status) is configured using the `namespace_storage` section. This is disctinct from the Namespaces configuration, which defines initial conditions for a namespace. You can choose from:

##### In-Memory Namespace Storage:
```toml
[namespace_storage]
type = "InMemory"
```

##### File-based Namespace Storage:
```toml
[namespace_storage]
type = "File"
```

#### Storage Configuration

The storage backend is configured using the `storage` section, which specifies how signatures should be persisted. You can choose from:

##### In-Memory Storage:
```toml
[signature_storage]
type = "InMemory"
```

##### File-based Storage:
```toml
[signature_storage]
type = "File" 
```

When using file-based storage:
  ```
  /var/lib/akd-watch/storage/signatures/namespace_name/
  ├── 1/
  │   └── sig
  ├── 2/
  │   └── sig
  └── ...
  ```
  Each signature file contains a protobuf serialization of the complete signature.

##### Azure Blob Storage: (coming soon)
```toml
[signature_storage]
type = "Azure"
account_name = "your_storage_account"
container_name = "your_container"
connection_string = "your_connection_string"  # Optional in config file
```

**Note:** Azure storage requires a connection string either in the config file or via the `AKD_WATCH__SIGNATURE_STORAGE__CONNECTION_STRING` environment variable. The configuration will be validated at startup to ensure the connection string is available from one of these sources.

#### Signing Configuration

The signing key configuration:
- `key_lifetime_seconds`: Lifetime of the signing key in seconds (defaults to 30 days)

#### Namespace Configuration

Each namespace requires:
- `name`: Unique namespace identifier
- `configuration_type`: Either "WhatsAppV1" or "BitwardenV1"
- `log_directory`: Url to query for AKD proofs
- `starting_epoch` (optional): Epoch to start auditing from (defaults to 0, only used if namespace doesn't already exist in repository)
- `status`: Either "Online" or "Disabled"

**Status Changes**:
**Error states are preserved.** If a namespace is in `SignatureLost` or `SignatureVerificationFailed` state, the configuration cannot override it. These states indicate that there is either an issue with signature storage (`SignatureLost`) or the directory being audited failed an audit (`SignatureVerificationFailed`). Directories that are happily running can be disabled or enabled via configuration.

### Environment Variables

You can override any configuration value using environment variables with the `AKD_WATCH__` prefix:

```bash
export AKD_WATCH__SLEEP_SECONDS=60
export AKD_WATCH__NAMESPACES__0__NAME="my_namespace"
export AKD_WATCH__NAMESPACES__0__CONFIGURATION_TYPE="BitwardenV1"
export AKD_WATCH__NAMESPACES__0__STARTING_EPOCH=5
```

### Usage

The auditor will:
1. Look for `config.toml`, `config.yaml`, or `config.json` in the current directory or in the configured path specified by the `AKD_WATCH_CONFIG_PATH` environment variable
2. Apply any environment variable overrides
3. Fall back to defaults for non-required settings


# steampipe-forge

Steampipe plugin code generator. Implements `iac_forge::Backend` to produce
Go table definitions and plugin registration code from the iac-forge IR.

## Architecture

Takes `IacResource` and `IacProvider` from iac-forge IR and generates complete
Steampipe plugin Go source files following the patterns established by the
`steampipe-plugin-akeyless` reference implementation.

Each resource becomes a table file with column definitions and a list hydrate
function stub. The provider-level artifact is `plugin.go` with the full
`TableMap` registration.

Steampipe tables are read-only -- there are no data source or CRUD concepts
beyond the list hydrate. Test artifacts produce basic query validation stubs.

## Generated File Structure

```
table_<provider>_<name>.go    -- per-resource table definition
plugin.go                     -- provider-level TableMap registration
table_<provider>_<name>_test.go -- per-resource test stub
```

## Type Mappings (IacType -> Steampipe proto.ColumnType)

```
IacType::String       -> proto.ColumnType_STRING
IacType::Integer      -> proto.ColumnType_INT
IacType::Float        -> proto.ColumnType_DOUBLE
IacType::Boolean      -> proto.ColumnType_BOOL
IacType::List(T)      -> proto.ColumnType_JSON
IacType::Set(T)       -> proto.ColumnType_JSON
IacType::Map(T)       -> proto.ColumnType_JSON
IacType::Object       -> proto.ColumnType_JSON
IacType::Enum         -> proto.ColumnType_STRING (underlying type governs)
IacType::Any          -> proto.ColumnType_JSON
```

## Key Types

- `SteampipeBackend` -- implements `iac_forge::Backend` trait (unit struct)
- `SteampipeNaming` -- naming convention: PascalCase types, snake_case files/fields

## Source Layout

```
src/
  lib.rs        -- Public API re-exports (SteampipeBackend)
  backend.rs    -- Backend trait implementation + naming convention
  table_gen.rs  -- Go table/plugin code generation
```

## Usage

```rust
use steampipe_forge::SteampipeBackend;
use iac_forge::Backend;

let backend = SteampipeBackend;
let artifacts = backend.generate_resource(&resource, &provider)?;
// artifacts[0].content is the Go table file
// artifacts[0].path is e.g. "table_akeyless_static_secret.go"
```

## Testing

Run: `cargo test`

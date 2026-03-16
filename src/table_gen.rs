//! Go code generation for Steampipe plugin tables.
//!
//! Generates table definition files and plugin registration code following the
//! patterns established by the steampipe-plugin-akeyless reference implementation.

use iac_forge::ir::{IacAttribute, IacDataSource, IacProvider, IacResource, IacType};
use iac_forge::naming::{to_pascal_case, to_snake_case};

/// Map an `IacType` to the Steampipe `proto.ColumnType_*` constant.
#[must_use]
pub fn iac_type_to_column_type(iac_type: &IacType) -> &'static str {
    match iac_type {
        IacType::String => "proto.ColumnType_STRING",
        IacType::Integer => "proto.ColumnType_INT",
        IacType::Float => "proto.ColumnType_DOUBLE",
        IacType::Boolean => "proto.ColumnType_BOOL",
        IacType::List(_) | IacType::Set(_) | IacType::Map(_) | IacType::Object { .. } | IacType::Any => {
            "proto.ColumnType_JSON"
        }
        IacType::Enum { underlying, .. } => iac_type_to_column_type(underlying),
    }
}

/// Generate the Go table definition file for a single resource.
///
/// Produces a file like `table_akeyless_static_secret.go` with the table
/// function, columns function, and a list hydrate stub.
#[must_use]
pub fn generate_table_file(resource: &IacResource, provider: &IacProvider) -> String {
    let snake_name = to_snake_case(&resource.name);
    let table_name = format!("{}_{}", provider.name, snake_name);
    let pascal_name = to_pascal_case(&resource.name);
    let provider_pascal = to_pascal_case(&provider.name);

    let description = if resource.description.is_empty() {
        format!("{} {} table", provider_pascal, pascal_name)
    } else {
        resource.description.clone()
    };

    let columns = generate_columns(&resource.attributes);

    format!(
        r#"package {provider_name}

import (
	"context"

	"github.com/turbot/steampipe-plugin-sdk/v5/grpc/proto"
	"github.com/turbot/steampipe-plugin-sdk/v5/plugin"
)

func table{provider_pascal}{pascal_name}() *plugin.Table {{
	return &plugin.Table{{
		Name:        "{table_name}",
		Description: "{description}",
		List: &plugin.ListConfig{{
			Hydrate: list{provider_pascal}{pascal_name},
		}},
		Columns: {provider_pascal_lower}{pascal_name}Columns(),
	}}
}}

func {provider_pascal_lower}{pascal_name}Columns() []*plugin.Column {{
	return []*plugin.Column{{
{columns}	}}
}}

func list{provider_pascal}{pascal_name}(ctx context.Context, d *plugin.QueryData, _ *plugin.HydrateData) (interface{{}}, error) {{
	// TODO: Implement list hydrate function.
	return nil, nil
}}
"#,
        provider_name = provider.name,
        provider_pascal = provider_pascal,
        provider_pascal_lower = lowercase_first(&provider_pascal),
        pascal_name = pascal_name,
        table_name = table_name,
        description = escape_go_string(&description),
        columns = columns,
    )
}

/// Generate the Go table definition file for a data source (read-only query).
///
/// Steampipe tables are inherently read-only, so data sources map naturally
/// to the same table pattern as resources.
#[must_use]
pub fn generate_data_source_table_file(ds: &IacDataSource, provider: &IacProvider) -> String {
    let snake_name = to_snake_case(&ds.name);
    let table_name = format!("{}_{}", provider.name, snake_name);
    let pascal_name = to_pascal_case(&ds.name);
    let provider_pascal = to_pascal_case(&provider.name);

    let description = if ds.description.is_empty() {
        format!("{} {} table", provider_pascal, pascal_name)
    } else {
        ds.description.clone()
    };

    let columns = generate_columns(&ds.attributes);

    format!(
        r#"package {provider_name}

import (
	"context"

	"github.com/turbot/steampipe-plugin-sdk/v5/grpc/proto"
	"github.com/turbot/steampipe-plugin-sdk/v5/plugin"
)

func table{provider_pascal}{pascal_name}() *plugin.Table {{
	return &plugin.Table{{
		Name:        "{table_name}",
		Description: "{description}",
		List: &plugin.ListConfig{{
			Hydrate: list{provider_pascal}{pascal_name},
		}},
		Columns: {provider_pascal_lower}{pascal_name}Columns(),
	}}
}}

func {provider_pascal_lower}{pascal_name}Columns() []*plugin.Column {{
	return []*plugin.Column{{
{columns}	}}
}}

func list{provider_pascal}{pascal_name}(ctx context.Context, d *plugin.QueryData, _ *plugin.HydrateData) (interface{{}}, error) {{
	// TODO: Implement list hydrate function.
	return nil, nil
}}
"#,
        provider_name = provider.name,
        provider_pascal = provider_pascal,
        provider_pascal_lower = lowercase_first(&provider_pascal),
        pascal_name = pascal_name,
        table_name = table_name,
        description = escape_go_string(&description),
        columns = columns,
    )
}

/// Generate `plugin.go` with the `TableMap` registration for all resources and data sources.
#[must_use]
pub fn generate_plugin_file(
    provider: &IacProvider,
    resources: &[IacResource],
    data_sources: &[IacDataSource],
) -> String {
    let provider_pascal = to_pascal_case(&provider.name);
    let plugin_name = format!("steampipe-plugin-{}", provider.name);

    let mut table_entries = Vec::new();

    for resource in resources {
        let snake_name = to_snake_case(&resource.name);
        let table_name = format!("{}_{}", provider.name, snake_name);
        let pascal_name = to_pascal_case(&resource.name);
        table_entries.push(format!(
            "\t\t\t\"{table_name}\": table{provider_pascal}{pascal_name}(),"
        ));
    }

    for ds in data_sources {
        let snake_name = to_snake_case(&ds.name);
        let table_name = format!("{}_{}", provider.name, snake_name);
        let pascal_name = to_pascal_case(&ds.name);
        table_entries.push(format!(
            "\t\t\t\"{table_name}\": table{provider_pascal}{pascal_name}(),"
        ));
    }

    let table_map = table_entries.join("\n");

    format!(
        r#"package {provider_name}

import (
	"context"

	"github.com/turbot/steampipe-plugin-sdk/v5/plugin"
)

func Plugin(ctx context.Context) *plugin.Plugin {{
	return &plugin.Plugin{{
		Name: "{plugin_name}",
		TableMap: map[string]*plugin.Table{{
{table_map}
		}},
	}}
}}
"#,
        provider_name = provider.name,
        plugin_name = plugin_name,
        table_map = table_map,
    )
}

/// Generate a basic test stub for a resource table.
#[must_use]
pub fn generate_test_file(resource: &IacResource, provider: &IacProvider) -> String {
    let snake_name = to_snake_case(&resource.name);
    let table_name = format!("{}_{}", provider.name, snake_name);
    let pascal_name = to_pascal_case(&resource.name);
    let provider_pascal = to_pascal_case(&provider.name);

    format!(
        r#"package {provider_name}

import (
	"testing"
)

func TestTable{provider_pascal}{pascal_name}(t *testing.T) {{
	table := table{provider_pascal}{pascal_name}()
	if table == nil {{
		t.Fatal("table{provider_pascal}{pascal_name}() returned nil")
	}}
	if table.Name != "{table_name}" {{
		t.Errorf("expected table name %q, got %q", "{table_name}", table.Name)
	}}
	if len(table.Columns) == 0 {{
		t.Error("expected at least one column")
	}}
}}
"#,
        provider_name = provider.name,
        provider_pascal = provider_pascal,
        pascal_name = pascal_name,
        table_name = table_name,
    )
}

/// Generate the column definitions for a list of attributes.
fn generate_columns(attributes: &[IacAttribute]) -> String {
    let mut lines = Vec::new();

    for attr in attributes {
        let col_type = iac_type_to_column_type(&attr.iac_type);
        let description = if attr.description.is_empty() {
            format!("The {} field.", attr.canonical_name)
        } else {
            attr.description.clone()
        };

        lines.push(format!(
            "\t\t{{\n\t\t\tName:        \"{name}\",\n\t\t\tType:        {col_type},\n\t\t\tDescription: \"{description}\",\n\t\t}},",
            name = attr.canonical_name,
            col_type = col_type,
            description = escape_go_string(&description),
        ));
    }

    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

/// Escape a string for use in a Go string literal (double-quoted).
fn escape_go_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

/// Lowercase the first character of a string.
fn lowercase_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => {
            let lower: String = c.to_lowercase().collect();
            format!("{lower}{}", chars.as_str())
        }
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iac_forge::testing::{
        TestAttributeBuilder, test_data_source, test_provider, test_resource,
        test_resource_with_type,
    };

    #[test]
    fn column_type_string() {
        assert_eq!(iac_type_to_column_type(&IacType::String), "proto.ColumnType_STRING");
    }

    #[test]
    fn column_type_integer() {
        assert_eq!(iac_type_to_column_type(&IacType::Integer), "proto.ColumnType_INT");
    }

    #[test]
    fn column_type_float() {
        assert_eq!(iac_type_to_column_type(&IacType::Float), "proto.ColumnType_DOUBLE");
    }

    #[test]
    fn column_type_boolean() {
        assert_eq!(iac_type_to_column_type(&IacType::Boolean), "proto.ColumnType_BOOL");
    }

    #[test]
    fn column_type_list() {
        assert_eq!(
            iac_type_to_column_type(&IacType::List(Box::new(IacType::String))),
            "proto.ColumnType_JSON"
        );
    }

    #[test]
    fn column_type_set() {
        assert_eq!(
            iac_type_to_column_type(&IacType::Set(Box::new(IacType::Integer))),
            "proto.ColumnType_JSON"
        );
    }

    #[test]
    fn column_type_map() {
        assert_eq!(
            iac_type_to_column_type(&IacType::Map(Box::new(IacType::String))),
            "proto.ColumnType_JSON"
        );
    }

    #[test]
    fn column_type_object() {
        assert_eq!(
            iac_type_to_column_type(&IacType::Object {
                name: "Cfg".to_string(),
                fields: vec![]
            }),
            "proto.ColumnType_JSON"
        );
    }

    #[test]
    fn column_type_any() {
        assert_eq!(iac_type_to_column_type(&IacType::Any), "proto.ColumnType_JSON");
    }

    #[test]
    fn column_type_enum_string() {
        assert_eq!(
            iac_type_to_column_type(&IacType::Enum {
                values: vec!["a".into()],
                underlying: Box::new(IacType::String),
            }),
            "proto.ColumnType_STRING"
        );
    }

    #[test]
    fn column_type_enum_integer() {
        assert_eq!(
            iac_type_to_column_type(&IacType::Enum {
                values: vec!["1".into()],
                underlying: Box::new(IacType::Integer),
            }),
            "proto.ColumnType_INT"
        );
    }

    #[test]
    fn generate_table_file_basic() {
        let provider = test_provider("akeyless");
        let resource = test_resource("secret");

        let code = generate_table_file(&resource, &provider);

        assert!(code.contains("package akeyless"));
        assert!(code.contains("func tableAkeylessSecret()"));
        assert!(code.contains("\"akeyless_secret\""));
        assert!(code.contains("akeylessSecretColumns()"));
        assert!(code.contains("listAkeylessSecret"));
        assert!(code.contains("proto.ColumnType_STRING"));
        assert!(code.contains("\"name\""));
        assert!(code.contains("\"value\""));
        assert!(code.contains("\"tags\""));
        // tags is List(String) -> JSON
        assert!(code.contains("proto.ColumnType_JSON"));
    }

    #[test]
    fn generate_table_file_boolean_type() {
        let provider = test_provider("acme");
        let resource = test_resource_with_type("flag", "enabled", IacType::Boolean);

        let code = generate_table_file(&resource, &provider);

        assert!(code.contains("package acme"));
        assert!(code.contains("func tableAcmeFlag()"));
        assert!(code.contains("proto.ColumnType_BOOL"));
        assert!(code.contains("\"enabled\""));
    }

    #[test]
    fn generate_table_file_integer_type() {
        let provider = test_provider("acme");
        let resource = test_resource_with_type("counter", "count", IacType::Integer);

        let code = generate_table_file(&resource, &provider);

        assert!(code.contains("proto.ColumnType_INT"));
    }

    #[test]
    fn generate_table_file_float_type() {
        let provider = test_provider("acme");
        let resource = test_resource_with_type("metric", "score", IacType::Float);

        let code = generate_table_file(&resource, &provider);

        assert!(code.contains("proto.ColumnType_DOUBLE"));
    }

    #[test]
    fn generate_table_file_map_type() {
        let provider = test_provider("acme");
        let resource = test_resource_with_type(
            "config",
            "settings",
            IacType::Map(Box::new(IacType::String)),
        );

        let code = generate_table_file(&resource, &provider);

        assert!(code.contains("proto.ColumnType_JSON"));
    }

    #[test]
    fn generate_plugin_file_basic() {
        let provider = test_provider("akeyless");
        let resources = vec![
            test_resource("secret"),
            test_resource_with_type("role", "name", IacType::String),
        ];

        let code = generate_plugin_file(&provider, &resources, &[]);

        assert!(code.contains("package akeyless"));
        assert!(code.contains("\"steampipe-plugin-akeyless\""));
        assert!(code.contains("\"akeyless_secret\": tableAkeylessSecret()"));
        assert!(code.contains("\"akeyless_role\": tableAkeylessRole()"));
        assert!(code.contains("func Plugin(ctx context.Context) *plugin.Plugin"));
    }

    #[test]
    fn generate_plugin_file_with_data_sources() {
        let provider = test_provider("acme");
        let resources = vec![test_resource("widget")];
        let data_sources = vec![test_data_source("config")];

        let code = generate_plugin_file(&provider, &resources, &data_sources);

        assert!(code.contains("\"acme_widget\": tableAcmeWidget()"));
        assert!(code.contains("\"acme_config\": tableAcmeConfig()"));
    }

    #[test]
    fn generate_plugin_file_empty() {
        let provider = test_provider("empty");
        let code = generate_plugin_file(&provider, &[], &[]);

        assert!(code.contains("\"steampipe-plugin-empty\""));
        assert!(code.contains("TableMap: map[string]*plugin.Table{"));
    }

    #[test]
    fn generate_test_file_basic() {
        let provider = test_provider("akeyless");
        let resource = test_resource("secret");

        let code = generate_test_file(&resource, &provider);

        assert!(code.contains("package akeyless"));
        assert!(code.contains("func TestTableAkeylessSecret(t *testing.T)"));
        assert!(code.contains("tableAkeylessSecret()"));
        assert!(code.contains("\"akeyless_secret\""));
    }

    #[test]
    fn generate_data_source_table_file_basic() {
        let provider = test_provider("acme");
        let ds = test_data_source("config");

        let code = generate_data_source_table_file(&ds, &provider);

        assert!(code.contains("package acme"));
        assert!(code.contains("func tableAcmeConfig()"));
        assert!(code.contains("\"acme_config\""));
        assert!(code.contains("acmeConfigColumns()"));
        assert!(code.contains("listAcmeConfig"));
    }

    #[test]
    fn pascal_case_naming_in_table() {
        let provider = test_provider("akeyless");
        let resource = test_resource_with_type("static_secret", "name", IacType::String);

        let code = generate_table_file(&resource, &provider);

        assert!(code.contains("func tableAkeylessStaticSecret()"));
        assert!(code.contains("akeylessStaticSecretColumns()"));
        assert!(code.contains("listAkeylessStaticSecret"));
    }

    #[test]
    fn escape_go_string_quotes() {
        assert_eq!(escape_go_string(r#"has "quotes""#), r#"has \"quotes\""#);
    }

    #[test]
    fn escape_go_string_newlines() {
        assert_eq!(escape_go_string("line1\nline2"), "line1\\nline2");
    }

    #[test]
    fn escape_go_string_backslash() {
        assert_eq!(escape_go_string(r"back\slash"), r"back\\slash");
    }

    #[test]
    fn lowercase_first_basic() {
        assert_eq!(lowercase_first("Akeyless"), "akeyless");
        assert_eq!(lowercase_first("ABC"), "aBC");
        assert_eq!(lowercase_first(""), "");
    }

    #[test]
    fn generate_table_with_all_types() {
        let provider = test_provider("acme");
        let mut resource = test_resource("all_types");

        resource.attributes = vec![
            TestAttributeBuilder::new("str_field", IacType::String).build(),
            TestAttributeBuilder::new("int_field", IacType::Integer).build(),
            TestAttributeBuilder::new("float_field", IacType::Float).build(),
            TestAttributeBuilder::new("bool_field", IacType::Boolean).build(),
            TestAttributeBuilder::new("list_field", IacType::List(Box::new(IacType::String))).build(),
            TestAttributeBuilder::new("set_field", IacType::Set(Box::new(IacType::Integer))).build(),
            TestAttributeBuilder::new("map_field", IacType::Map(Box::new(IacType::String))).build(),
            TestAttributeBuilder::new(
                "obj_field",
                IacType::Object {
                    name: "Config".to_string(),
                    fields: vec![],
                },
            )
            .build(),
            TestAttributeBuilder::new(
                "enum_field",
                IacType::Enum {
                    values: vec!["a".into(), "b".into()],
                    underlying: Box::new(IacType::String),
                },
            )
            .build(),
            TestAttributeBuilder::new("any_field", IacType::Any).build(),
        ];

        let code = generate_table_file(&resource, &provider);

        assert!(code.contains("\"str_field\""));
        assert!(code.contains("\"int_field\""));
        assert!(code.contains("\"float_field\""));
        assert!(code.contains("\"bool_field\""));
        assert!(code.contains("\"list_field\""));
        assert!(code.contains("\"set_field\""));
        assert!(code.contains("\"map_field\""));
        assert!(code.contains("\"obj_field\""));
        assert!(code.contains("\"enum_field\""));
        assert!(code.contains("\"any_field\""));

        // Verify correct type mappings
        assert!(code.contains("proto.ColumnType_STRING"));
        assert!(code.contains("proto.ColumnType_INT"));
        assert!(code.contains("proto.ColumnType_DOUBLE"));
        assert!(code.contains("proto.ColumnType_BOOL"));
        assert!(code.contains("proto.ColumnType_JSON"));
    }

    #[test]
    fn generate_table_empty_attributes() {
        let provider = test_provider("acme");
        let mut resource = test_resource("empty");
        resource.attributes = vec![];

        let code = generate_table_file(&resource, &provider);

        assert!(code.contains("func tableAcmeEmpty()"));
        assert!(code.contains("return []*plugin.Column{"));
    }

    #[test]
    fn generate_plugin_multiple_resources_ordered() {
        let provider = test_provider("akeyless");
        let resources = vec![
            test_resource("auth_method"),
            test_resource("gateway"),
            test_resource("item"),
            test_resource("role"),
            test_resource("target"),
        ];

        let code = generate_plugin_file(&provider, &resources, &[]);

        assert!(code.contains("\"akeyless_auth_method\": tableAkeylessAuthMethod()"));
        assert!(code.contains("\"akeyless_gateway\": tableAkeylessGateway()"));
        assert!(code.contains("\"akeyless_item\": tableAkeylessItem()"));
        assert!(code.contains("\"akeyless_role\": tableAkeylessRole()"));
        assert!(code.contains("\"akeyless_target\": tableAkeylessTarget()"));
    }
}

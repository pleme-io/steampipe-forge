//! Go code generation for Steampipe plugin tables.
//!
//! Generates table definition files and plugin registration code following the
//! patterns established by the steampipe-plugin-akeyless reference implementation.

use std::fmt;

use iac_forge::ir::{IacAttribute, IacDataSource, IacProvider, IacResource, IacType};
use iac_forge::naming::{to_pascal_case, to_snake_case};

/// Pre-computed naming components used throughout table code generation.
///
/// Centralises the `snake_case` / `PascalCase` derivations so every
/// generator works from one consistent set of names.
#[derive(Debug, Clone)]
struct TableNames {
    /// e.g. `"akeyless"`
    provider_name: String,
    /// e.g. `"Akeyless"`
    provider_pascal: String,
    /// e.g. `"akeyless_static_secret"`
    table_name: String,
    /// e.g. `"StaticSecret"`
    pascal_name: String,
}

impl TableNames {
    fn new(entity_name: &str, provider: &IacProvider) -> Self {
        let snake_name = to_snake_case(entity_name);
        Self {
            provider_name: provider.name.clone(),
            provider_pascal: to_pascal_case(&provider.name),
            table_name: format!("{}_{snake_name}", provider.name),
            pascal_name: to_pascal_case(entity_name),
        }
    }

    /// Fallback description when the entity has none of its own.
    fn default_description(&self) -> String {
        format!("{} {} table", self.provider_pascal, self.pascal_name)
    }
}

/// Steampipe column type mapped from the IR type system.
///
/// Wraps the Go `proto.ColumnType_*` constant string so we can provide
/// idiomatic [`From`] and [`Display`] implementations instead of a bare
/// function returning `&'static str`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnType(&'static str);

impl ColumnType {
    /// The underlying Go constant string (e.g. `"proto.ColumnType_STRING"`).
    #[must_use]
    pub fn as_go_const(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for ColumnType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl From<&IacType> for ColumnType {
    fn from(iac_type: &IacType) -> Self {
        match iac_type {
            IacType::String => Self("proto.ColumnType_STRING"),
            IacType::Integer => Self("proto.ColumnType_INT"),
            IacType::Float => Self("proto.ColumnType_DOUBLE"),
            IacType::Boolean => Self("proto.ColumnType_BOOL"),
            IacType::List(_)
            | IacType::Set(_)
            | IacType::Map(_)
            | IacType::Object { .. }
            | IacType::Any => Self("proto.ColumnType_JSON"),
            IacType::Enum { underlying, .. } => Self::from(underlying.as_ref()),
        }
    }
}

/// Common interface for IR entities that map to a Steampipe table.
///
/// Both [`IacResource`] and [`IacDataSource`] carry a name, description,
/// and attribute list -- everything needed to emit a Go table file. This
/// trait captures that overlap so a single `generate_table` function can
/// serve both callers.
pub(crate) trait TableSource {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn attributes(&self) -> &[IacAttribute];
}

impl TableSource for IacResource {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn attributes(&self) -> &[IacAttribute] {
        &self.attributes
    }
}

impl TableSource for IacDataSource {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn attributes(&self) -> &[IacAttribute] {
        &self.attributes
    }
}

/// Generate the Go table definition file for any [`TableSource`].
fn generate_table(source: &dyn TableSource, provider: &IacProvider) -> String {
    let names = TableNames::new(source.name(), provider);

    let desc_raw = source.description();
    let description = if desc_raw.is_empty() {
        names.default_description()
    } else {
        desc_raw.to_owned()
    };

    let columns = generate_columns(source.attributes());
    format_table_go(&names, &description, &columns)
}

/// Map an `IacType` to the Steampipe `proto.ColumnType_*` constant.
///
/// Prefer using `ColumnType::from(iac_type)` directly for richer type
/// information; this function is kept for backward compatibility.
#[must_use]
pub fn iac_type_to_column_type(iac_type: &IacType) -> &'static str {
    ColumnType::from(iac_type).as_go_const()
}

/// Generate the Go table definition file for a single resource.
///
/// Produces a file like `table_akeyless_static_secret.go` with the table
/// function, columns function, and a list hydrate stub.
#[must_use]
pub fn generate_table_file(resource: &IacResource, provider: &IacProvider) -> String {
    generate_table(resource, provider)
}

/// Generate the Go table definition file for a data source (read-only query).
///
/// Steampipe tables are inherently read-only, so data sources map naturally
/// to the same table pattern as resources.
#[must_use]
pub fn generate_data_source_table_file(ds: &IacDataSource, provider: &IacProvider) -> String {
    generate_table(ds, provider)
}

/// Format the Go table definition source for a single table.
///
/// Shared by both resource and data-source table generation to avoid
/// duplicating the Go template.
fn format_table_go(names: &TableNames, description: &str, columns: &str) -> String {
    let provider_name = &names.provider_name;
    let provider_pascal = &names.provider_pascal;
    let pascal_name = &names.pascal_name;
    let table_name = &names.table_name;
    let provider_pascal_lower = lowercase_first(provider_pascal);
    let escaped_description = escape_go_string(description);

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
		Description: "{escaped_description}",
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
"#
    )
}

/// Generate `plugin.go` with the `TableMap` registration for all resources and data sources.
#[must_use]
pub fn generate_plugin_file(
    provider: &IacProvider,
    resources: &[IacResource],
    data_sources: &[IacDataSource],
) -> String {
    let plugin_name = format!("steampipe-plugin-{}", provider.name);

    let table_map: String = resources
        .iter()
        .map(|r| &r.name)
        .chain(data_sources.iter().map(|d| &d.name))
        .map(|name| format_table_map_entry(&TableNames::new(name, provider)))
        .collect::<Vec<_>>()
        .join("\n");

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
    )
}

/// Format a single `TableMap` entry line for `plugin.go`.
fn format_table_map_entry(names: &TableNames) -> String {
    format!(
        "\t\t\t\"{}\": table{}{}(),",
        names.table_name, names.provider_pascal, names.pascal_name
    )
}

/// Generate a basic test stub for a resource table.
#[must_use]
pub fn generate_test_file(resource: &IacResource, provider: &IacProvider) -> String {
    let names = TableNames::new(&resource.name, provider);

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
        provider_name = names.provider_name,
        provider_pascal = names.provider_pascal,
        pascal_name = names.pascal_name,
        table_name = names.table_name,
    )
}

/// Generate the column definitions for a list of attributes.
fn generate_columns(attributes: &[IacAttribute]) -> String {
    if attributes.is_empty() {
        return String::new();
    }

    let body: String = attributes
        .iter()
        .map(|attr| {
            let col_type = ColumnType::from(&attr.iac_type);
            let desc = if attr.description.is_empty() {
                format!("The {} field.", attr.canonical_name)
            } else {
                attr.description.clone()
            };
            format!(
                "\t\t{{\n\t\t\tName:        \"{name}\",\n\t\t\tType:        {col_type},\n\t\t\tDescription: \"{desc}\",\n\t\t}},",
                name = attr.canonical_name,
                desc = escape_go_string(&desc),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!("{body}\n")
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
    fn column_type_newtype_from_iac_type() {
        let ct = ColumnType::from(&IacType::String);
        assert_eq!(ct.as_go_const(), "proto.ColumnType_STRING");

        let ct = ColumnType::from(&IacType::Integer);
        assert_eq!(ct.as_go_const(), "proto.ColumnType_INT");
    }

    #[test]
    fn column_type_display() {
        let ct = ColumnType::from(&IacType::Boolean);
        assert_eq!(ct.to_string(), "proto.ColumnType_BOOL");
    }

    #[test]
    fn column_type_equality() {
        let a = ColumnType::from(&IacType::Float);
        let b = ColumnType::from(&IacType::Float);
        assert_eq!(a, b);

        let c = ColumnType::from(&IacType::String);
        assert_ne!(a, c);
    }

    #[test]
    fn table_names_default_description() {
        let provider = test_provider("acme");
        let names = TableNames::new("static_secret", &provider);
        assert_eq!(names.default_description(), "Acme StaticSecret table");
    }

    #[test]
    fn table_source_resource_impl() {
        let resource = test_resource("secret");
        let source: &dyn TableSource = &resource;
        assert_eq!(source.name(), "secret");
        assert!(!source.attributes().is_empty());
    }

    #[test]
    fn table_source_data_source_impl() {
        let ds = test_data_source("config");
        let source: &dyn TableSource = &ds;
        assert_eq!(source.name(), "config");
        assert!(!source.attributes().is_empty());
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
    fn escape_go_string_tabs() {
        assert_eq!(escape_go_string("col1\tcol2"), "col1\\tcol2");
    }

    #[test]
    fn escape_go_string_combined() {
        assert_eq!(
            escape_go_string("line1\nhas \"quotes\" and\ttabs\\end"),
            r#"line1\nhas \"quotes\" and\ttabs\\end"#
        );
    }

    #[test]
    fn escape_go_string_empty() {
        assert_eq!(escape_go_string(""), "");
    }

    #[test]
    fn escape_go_string_no_special_chars() {
        assert_eq!(escape_go_string("plain text"), "plain text");
    }

    #[test]
    fn lowercase_first_basic() {
        assert_eq!(lowercase_first("Akeyless"), "akeyless");
        assert_eq!(lowercase_first("ABC"), "aBC");
        assert_eq!(lowercase_first(""), "");
    }

    #[test]
    fn lowercase_first_already_lower() {
        assert_eq!(lowercase_first("already"), "already");
    }

    #[test]
    fn lowercase_first_single_char() {
        assert_eq!(lowercase_first("X"), "x");
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
    fn generate_table_file_custom_description() {
        let provider = test_provider("acme");
        let mut resource = test_resource_with_type("widget", "name", IacType::String);
        resource.description = "Custom widget description with \"quotes\"".to_string();

        let code = generate_table_file(&resource, &provider);

        assert!(code.contains(r#"Description: "Custom widget description with \"quotes\"""#));
    }

    #[test]
    fn generate_data_source_custom_description() {
        let provider = test_provider("acme");
        let mut ds = test_data_source("config");
        ds.description = "Custom config data source".to_string();

        let code = generate_data_source_table_file(&ds, &provider);

        assert!(code.contains(r#"Description: "Custom config data source""#));
    }

    #[test]
    fn generate_columns_with_custom_attr_description() {
        let provider = test_provider("acme");
        let mut resource = test_resource("widget");
        resource.attributes = vec![
            TestAttributeBuilder::new("id", IacType::String)
                .description("Unique identifier for the widget")
                .build(),
        ];

        let code = generate_table_file(&resource, &provider);

        assert!(code.contains(r#"Description: "Unique identifier for the widget""#));
    }

    #[test]
    fn generate_columns_default_attr_description() {
        let provider = test_provider("acme");
        let mut resource = test_resource("widget");
        resource.attributes = vec![
            TestAttributeBuilder::new("count", IacType::Integer).build(),
        ];

        let code = generate_table_file(&resource, &provider);

        assert!(code.contains(r#"Description: "The count field.""#));
    }

    #[test]
    fn generate_test_file_multi_word_resource() {
        let provider = test_provider("akeyless");
        let resource = test_resource_with_type("static_secret", "name", IacType::String);

        let code = generate_test_file(&resource, &provider);

        assert!(code.contains("func TestTableAkeylessStaticSecret(t *testing.T)"));
        assert!(code.contains("tableAkeylessStaticSecret()"));
        assert!(code.contains("\"akeyless_static_secret\""));
    }

    #[test]
    fn generate_data_source_table_matches_resource_structure() {
        let provider = test_provider("acme");
        let ds = test_data_source("config");
        let code = generate_data_source_table_file(&ds, &provider);

        assert!(code.contains("import ("));
        assert!(code.contains("\"github.com/turbot/steampipe-plugin-sdk/v5/grpc/proto\""));
        assert!(code.contains("\"github.com/turbot/steampipe-plugin-sdk/v5/plugin\""));
        assert!(code.contains("*plugin.Table"));
        assert!(code.contains("*plugin.Column"));
    }

    #[test]
    fn generate_plugin_file_only_data_sources() {
        let provider = test_provider("acme");
        let data_sources = vec![
            test_data_source("users"),
            test_data_source("groups"),
        ];

        let code = generate_plugin_file(&provider, &[], &data_sources);

        assert!(code.contains("\"acme_users\": tableAcmeUsers()"));
        assert!(code.contains("\"acme_groups\": tableAcmeGroups()"));
        assert!(!code.contains("tableAcme()"));
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

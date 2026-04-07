use iac_forge::backend::{ArtifactKind, Backend, GeneratedArtifact, NamingConvention};
use iac_forge::error::IacForgeError;
use iac_forge::ir::{IacDataSource, IacProvider, IacResource};
use iac_forge::naming::to_snake_case;

use crate::table_gen;

/// Steampipe backend -- generates Go table definitions from `IaC` forge IR.
pub struct SteampipeBackend;

/// Naming convention for Steampipe plugin tables.
struct SteampipeNaming;

impl NamingConvention for SteampipeNaming {
    fn resource_type_name(&self, resource_name: &str, provider_name: &str) -> String {
        format!("{}_{}", provider_name, to_snake_case(resource_name))
    }

    fn file_name(&self, resource_name: &str, kind: &ArtifactKind) -> String {
        let snake = to_snake_case(resource_name);
        match kind {
            ArtifactKind::Resource | ArtifactKind::DataSource => {
                format!("table_{snake}.go")
            }
            ArtifactKind::Test => format!("table_{snake}_test.go"),
            ArtifactKind::Provider => "plugin.go".to_string(),
            _ => format!("{snake}.go"),
        }
    }

    fn field_name(&self, api_name: &str) -> String {
        to_snake_case(api_name)
    }
}

impl Backend for SteampipeBackend {
    // TODO(scope): upstream Backend trait should use `&'static str` for platform()
    #[allow(clippy::unnecessary_literal_bound)]
    fn platform(&self) -> &str {
        "steampipe"
    }

    fn generate_resource(
        &self,
        resource: &IacResource,
        provider: &IacProvider,
    ) -> Result<Vec<GeneratedArtifact>, IacForgeError> {
        let content = table_gen::generate_table_file(resource, provider);
        let snake_name = to_snake_case(&resource.name);
        let path = format!("table_{}_{}.go", provider.name, snake_name);

        Ok(vec![GeneratedArtifact {
            path,
            content,
            kind: ArtifactKind::Resource,
        }])
    }

    fn generate_data_source(
        &self,
        ds: &IacDataSource,
        provider: &IacProvider,
    ) -> Result<Vec<GeneratedArtifact>, IacForgeError> {
        let content = table_gen::generate_data_source_table_file(ds, provider);
        let snake_name = to_snake_case(&ds.name);
        let path = format!("table_{}_{}.go", provider.name, snake_name);

        Ok(vec![GeneratedArtifact {
            path,
            content,
            kind: ArtifactKind::DataSource,
        }])
    }

    fn generate_provider(
        &self,
        provider: &IacProvider,
        resources: &[IacResource],
        data_sources: &[IacDataSource],
    ) -> Result<Vec<GeneratedArtifact>, IacForgeError> {
        let content = table_gen::generate_plugin_file(provider, resources, data_sources);

        Ok(vec![GeneratedArtifact {
            path: "plugin.go".to_string(),
            content,
            kind: ArtifactKind::Provider,
        }])
    }

    fn generate_test(
        &self,
        resource: &IacResource,
        provider: &IacProvider,
    ) -> Result<Vec<GeneratedArtifact>, IacForgeError> {
        let content = table_gen::generate_test_file(resource, provider);
        let snake_name = to_snake_case(&resource.name);
        let path = format!("table_{}_{}_test.go", provider.name, snake_name);

        Ok(vec![GeneratedArtifact {
            path,
            content,
            kind: ArtifactKind::Test,
        }])
    }

    fn naming(&self) -> &dyn NamingConvention {
        &SteampipeNaming
    }

    fn validate_resource(
        &self,
        resource: &IacResource,
        _provider: &IacProvider,
    ) -> Vec<String> {
        let mut warnings = Vec::new();
        if resource.attributes.is_empty() {
            warnings.push(format!(
                "resource '{}' has no attributes -- table will have no columns",
                resource.name
            ));
        }
        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iac_forge::ir::IacType;
    use iac_forge::testing::{
        test_data_source, test_provider, test_resource,
        test_resource_with_type,
    };

    #[test]
    fn platform_name() {
        let backend = SteampipeBackend;
        assert_eq!(backend.platform(), "steampipe");
    }

    #[test]
    fn naming_resource_type_name() {
        let naming = SteampipeNaming;
        assert_eq!(
            naming.resource_type_name("static_secret", "akeyless"),
            "akeyless_static_secret"
        );
    }

    #[test]
    fn naming_file_name_resource() {
        let naming = SteampipeNaming;
        assert_eq!(
            naming.file_name("akeyless_secret", &ArtifactKind::Resource),
            "table_akeyless_secret.go"
        );
    }

    #[test]
    fn naming_file_name_test() {
        let naming = SteampipeNaming;
        assert_eq!(
            naming.file_name("akeyless_secret", &ArtifactKind::Test),
            "table_akeyless_secret_test.go"
        );
    }

    #[test]
    fn naming_file_name_provider() {
        let naming = SteampipeNaming;
        assert_eq!(
            naming.file_name("akeyless", &ArtifactKind::Provider),
            "plugin.go"
        );
    }

    #[test]
    fn naming_field_name() {
        let naming = SteampipeNaming;
        assert_eq!(naming.field_name("bound-aws-account-id"), "bound_aws_account_id");
    }

    #[test]
    fn generate_resource_produces_artifact() {
        let backend = SteampipeBackend;
        let provider = test_provider("akeyless");
        let resource = test_resource("secret");

        let artifacts = backend
            .generate_resource(&resource, &provider)
            .expect("generate");
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].kind, ArtifactKind::Resource);
        assert_eq!(artifacts[0].path, "table_akeyless_secret.go");
        assert!(artifacts[0].content.contains("package akeyless"));
        assert!(artifacts[0].content.contains("tableAkeylessSecret"));
    }

    #[test]
    fn generate_data_source_produces_artifact() {
        let backend = SteampipeBackend;
        let provider = test_provider("acme");
        let ds = test_data_source("config");

        let artifacts = backend
            .generate_data_source(&ds, &provider)
            .expect("generate");
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].kind, ArtifactKind::DataSource);
        assert_eq!(artifacts[0].path, "table_acme_config.go");
        assert!(artifacts[0].content.contains("tableAcmeConfig"));
    }

    #[test]
    fn generate_provider_produces_plugin_go() {
        let backend = SteampipeBackend;
        let provider = test_provider("akeyless");
        let resources = vec![test_resource("secret"), test_resource("role")];

        let artifacts = backend
            .generate_provider(&provider, &resources, &[])
            .expect("generate");
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].kind, ArtifactKind::Provider);
        assert_eq!(artifacts[0].path, "plugin.go");
        assert!(artifacts[0].content.contains("steampipe-plugin-akeyless"));
        assert!(artifacts[0].content.contains("akeyless_secret"));
        assert!(artifacts[0].content.contains("akeyless_role"));
    }

    #[test]
    fn generate_test_produces_artifact() {
        let backend = SteampipeBackend;
        let provider = test_provider("akeyless");
        let resource = test_resource("secret");

        let artifacts = backend
            .generate_test(&resource, &provider)
            .expect("generate");
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].kind, ArtifactKind::Test);
        assert_eq!(artifacts[0].path, "table_akeyless_secret_test.go");
        assert!(artifacts[0].content.contains("TestTableAkeylessSecret"));
    }

    #[test]
    fn generate_all_produces_full_set() {
        let backend = SteampipeBackend;
        let provider = test_provider("akeyless");
        let resources = vec![test_resource("secret"), test_resource("role")];
        let data_sources = vec![test_data_source("config")];

        let artifacts = backend
            .generate_all(&provider, &resources, &data_sources)
            .expect("generate_all");

        // 2 resources + 1 data source + 1 plugin.go + 2 tests = 6
        assert_eq!(artifacts.len(), 6);
        assert_eq!(
            artifacts.iter().filter(|a| a.kind == ArtifactKind::Resource).count(),
            2
        );
        assert_eq!(
            artifacts.iter().filter(|a| a.kind == ArtifactKind::DataSource).count(),
            1
        );
        assert_eq!(
            artifacts.iter().filter(|a| a.kind == ArtifactKind::Provider).count(),
            1
        );
        assert_eq!(
            artifacts.iter().filter(|a| a.kind == ArtifactKind::Test).count(),
            2
        );
    }

    #[test]
    fn validate_resource_empty_attributes_warns() {
        let backend = SteampipeBackend;
        let provider = test_provider("acme");
        let mut resource = test_resource("empty");
        resource.attributes = vec![];

        let warnings = backend.validate_resource(&resource, &provider);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("no attributes"));
    }

    #[test]
    fn validate_resource_with_attributes_ok() {
        let backend = SteampipeBackend;
        let provider = test_provider("acme");
        let resource = test_resource("secret");

        let warnings = backend.validate_resource(&resource, &provider);
        assert!(warnings.is_empty());
    }

    #[test]
    fn generate_resource_all_types() {
        let backend = SteampipeBackend;
        let provider = test_provider("acme");

        // Test each IacType variant
        let types_and_expected: Vec<(&str, IacType, &str)> = vec![
            ("str_res", IacType::String, "proto.ColumnType_STRING"),
            ("int_res", IacType::Integer, "proto.ColumnType_INT"),
            ("float_res", IacType::Float, "proto.ColumnType_DOUBLE"),
            ("bool_res", IacType::Boolean, "proto.ColumnType_BOOL"),
            (
                "list_res",
                IacType::List(Box::new(IacType::String)),
                "proto.ColumnType_JSON",
            ),
            (
                "map_res",
                IacType::Map(Box::new(IacType::String)),
                "proto.ColumnType_JSON",
            ),
        ];

        for (name, iac_type, expected_col) in types_and_expected {
            let resource = test_resource_with_type(name, "field", iac_type);
            let artifacts = backend.generate_resource(&resource, &provider).expect("generate");
            assert!(
                artifacts[0].content.contains(expected_col),
                "resource {name} should contain {expected_col}"
            );
        }
    }

    #[test]
    fn generate_resource_pascal_case_naming() {
        let backend = SteampipeBackend;
        let provider = test_provider("akeyless");
        let resource = test_resource_with_type("static_secret", "name", IacType::String);

        let artifacts = backend.generate_resource(&resource, &provider).expect("generate");
        let content = &artifacts[0].content;

        assert!(content.contains("tableAkeylessStaticSecret"));
        assert!(content.contains("akeylessStaticSecretColumns"));
        assert!(content.contains("listAkeylessStaticSecret"));
    }

    #[test]
    fn naming_data_source_type_name_delegates() {
        let naming = SteampipeNaming;
        // data_source_type_name defaults to resource_type_name
        assert_eq!(
            naming.data_source_type_name("config", "acme"),
            naming.resource_type_name("config", "acme")
        );
    }
}

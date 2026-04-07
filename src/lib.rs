//! Steampipe plugin code generator from `IaC` forge IR.
//!
//! Implements [`iac_forge::backend::Backend`] to produce Go table definitions
//! and plugin registration code from the iac-forge intermediate representation.
//!
//! # Usage
//!
//! ```rust,ignore
//! use steampipe_forge::SteampipeBackend;
//! use iac_forge::backend::Backend;
//!
//! let backend = SteampipeBackend;
//! let artifacts = backend.generate_resource(&resource, &provider)?;
//! ```

pub mod backend;
pub(crate) mod table_gen;

pub use backend::SteampipeBackend;
pub use table_gen::{ColumnType, iac_type_to_column_type};

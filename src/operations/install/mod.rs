//! Complete install workflow orchestration for Augent bundles
//!
//! This module provides complete installation workflow, from bundle discovery
//! through dependency resolution to final resource installation and configuration updates.
//!
//! ## Architecture
//!
//! The install operation follows a modular coordinator pattern with specialized submodules:
//!
//! - **orchestrator**: Main workflow coordinator that coordinates all other submodules
//! - **resolution**: Install-specific bundle resolution coordinator
//! - **execution**: Execution orchestrator that performs actual file installation
//! - **workspace**: Workspace manager for workspace detection and modified file handling
//! - **config**: Config updater that writes augent.yaml, augent.lock, and augent.index.yaml
//! - **names**: Name fixer that ensures correct bundle naming conventions
//! - **lockfile**: Lockfile helpers for SHA tracking and hash verification
//! - **display**: Display utilities for user-facing output
//! - **context**: Shared context consolidating coordinator instances and common state
//!
//! ## Installation Workflow
//!
//! The install operation follows this sequence:
//!
//! ```text
//! 1. Bundle Discovery
//!    └─ Parse source (local path or git URL)
//!    └─ Discover available bundles in source
//!    └─ User selects bundles to install
//!
//! 2. Dependency Resolution
//!    └─ Load bundle configurations (augent.yaml)
//!    └─ Build dependency graph
//!    └─ Topological sort for installation order
//!    └─ Resolve git refs to SHAs
//!
//! 3. Workspace Preparation
//!    └─ Detect and preserve modified files
//!    └─ Prepare workspace bundle for execution
//!    └─ Detect or select target platforms
//!
//! 4. Installation Execution
//!    └─ Cache bundles (if git-based)
//!    └─ Transform resources to platform format
//!    └─ Merge files with existing content
//!    └─ Track installed files in transaction
//!
//! 5. Configuration Updates
//!    └─ Update augent.yaml (bundle dependencies)
//!    └─ Update augent.lock (resolved SHAs)
//!    └─ Update augent.index.yaml (installed file locations)
//! ```
//!
//! ## Coordinator Pattern
//!
//! Each major concern has a dedicated coordinator struct:
//!
//! - **InstallOperation**: Main workflow coordinator
//! - **InstallContext**: Shared context for all coordinators
//! - **InstallResolver**: Install-specific resolution coordinator
//!
//! Each major concern has a dedicated coordinator struct:
//!
//! - **InstallOperation**: Main workflow coordinator
//! - **InstallContext**: Shared context for all coordinators
//! - **InstallResolver**: Install-specific resolution coordinator
//! - **ExecutionOrchestrator**: Installation execution coordinator
//! - **WorkspaceManager**: Workspace operations coordinator
//! - **NameFixer**: Bundle name handling coordinator
//!
//! This pattern ensures clear separation of concerns and makes testing easier.
//!
//! ## Borrow Checker Management
//!
//! The install operation uses lifetime-patterned design to manage Rust's borrow checker:
//!
//! ```rust,ignore
//! pub struct InstallOperation<'a> {
//!     workspace: &'a mut Workspace,
//! }
//!
//! impl<'a> InstallOperation<'a> {
//!     pub fn execute(&mut self) -> Result<()> {
//!         // Immutable borrow phase
//!         let resolved_bundles = {
//!             let resolver = InstallResolver::new(self.workspace);
//!             resolver.resolve_selected_bundles()?
//!         };
//!
//!         // Mutable borrow phase
//!         let mut workspace_manager = WorkspaceManager::new(self.workspace);
//!         workspace_manager.detect_and_preserve_modified_files()?;
//!     }
//! }
//! ```
//!
//! This pattern allows multiple coordinators to access the workspace sequentially.
//!
//! ## Transaction Safety
//!
//! All file modifications are performed within a transaction:
//!
//! - Files are tracked before modification
//! - On error, all changes are rolled back
//! - Only committed changes persist
//! - Modified files are preserved across installations
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use augent::operations::install::InstallOperation;
//! use augent::workspace::Workspace;
//! use augent::cli::InstallArgs;
//!
//! // Parse arguments from CLI
//! let args = InstallArgs::parse();
//!
//! // Open workspace
//! let mut workspace = Workspace::init_or_open(&std::path::Path::new("."))?;
//!
//! // Create install operation
//! let options = InstallOptions::from(&args);
//! let mut install = InstallOperation::new(&mut workspace, options);
//!
//! // Execute installation
//! install.execute(&args)?;
//! ```

pub mod config;
pub mod context;
pub mod display;
pub mod execution;
pub mod lockfile;
pub mod names;
pub mod orchestrator;
pub mod resolution;
pub mod workspace;

pub use orchestrator::{InstallOperation, InstallOptions};

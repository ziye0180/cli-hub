/// Deep link import functionality for CLI Hub
///
/// This module implements the clihub:// protocol for importing provider configurations
/// via deep links. See docs/clihub-deeplink-design.md for detailed design.

pub mod types;
mod parser;
mod provider;
mod mcp;
mod prompt;
mod skill;
mod utils;

// Re-export public API
pub use types::*;
pub use parser::parse_deeplink_url;
pub use provider::{import_provider_from_deeplink, parse_and_merge_config};
pub use mcp::import_mcp_from_deeplink;
pub use prompt::import_prompt_from_deeplink;
pub use skill::import_skill_from_deeplink;

//! The JSON views of the studio's results, shared by `theos` and the MCP server (P48). Every key
//! is always present; an absent value is `null`.

pub mod check;
pub mod hook;
pub mod ledger;
pub mod new;
pub mod schema;

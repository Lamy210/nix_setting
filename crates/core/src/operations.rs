#[path = "operations_impl.rs"]
mod implementation;

pub use implementation::*;

/// Public source-initialization boundary.
///
/// Persisted managed-source state is authoritative. Validate it before the
/// implementation is allowed to discover remote tags, fetch release metadata,
/// or inspect the checkout. Invalid state therefore remains available for
/// explicit repair instead of being silently replaced by a new source.
pub fn source_init(
    repo: &str,
    store: &crate::state::StateStore,
    git: &crate::tool::ResolvedTool,
    channel: Option<String>,
    tag: Option<String>,
) -> crate::error::Result<SourceInitResult> {
    let state = store.load()?;
    if let Some(source) = state
        .as_ref()
        .and_then(|state| state.source.as_ref())
        .filter(|source| source.managed)
    {
        source.validate_managed_release()?;
    }

    implementation::source_init(repo, store, git, channel, tag)
}

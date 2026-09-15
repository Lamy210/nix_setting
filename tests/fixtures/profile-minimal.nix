# CI realization fixture: keep the default developer profile evaluated,
# but realize the lightweight profile to avoid rebuilding large upstream
# developer-only packages (notably Terraform) on every pull request.
{ profile = "minimal"; }

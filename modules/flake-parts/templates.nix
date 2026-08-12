_: {
  flake.templates = {
    rust = {
      path = ../../templates/rust;
      description = "Rust project (cargo + rustc + clippy + rust-analyzer)";
    };
    node = {
      path = ../../templates/node;
      description = "Node.js 24 project (pnpm + bun + TypeScript)";
    };
    python = {
      path = ../../templates/python;
      description = "Python project (uv + ruff + pyright)";
    };
    flutter = {
      path = ../../templates/flutter;
      description = "Flutter project (flutter + dart + jdk)";
    };
    devenv = {
      path = ../../templates/devenv;
      description = "Full-stack dev env (Go/Python/Node/Rust + PostgreSQL + Redis)";
    };
  };
}

Name:           oxide-embed
Version:        0.2.0
Release:        1%{?dist}
Summary:        Offline-first AST-aware context engine, GraphRAG memory, and MCP server for AI coding agents
License:        MIT OR Apache-2.0
URL:            https://github.com/FaezBarghasa/Oxide-embed
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust >= 1.85
BuildRequires:  cargo
BuildRequires:  systemd-rpm-macros

%description
Oxide-embed is a high-performance, 100% offline-first AST-aware context engine,
codebase GraphRAG memory system, and Model Context Protocol (MCP) server written
in pure Rust. It delivers sub-millisecond tree-sitter symbol parsing, zero-cloud
vector embeddings via Candle, and graph traversal with embedded SurrealDB.

%prep
%autosetup

%build
cargo build --release --locked

%install
rm -rf %{buildroot}
install -D -p -m 0755 target/release/oxide-embed %{buildroot}%{_bindir}/oxide-embed
install -D -p -m 0644 packaging/systemd/oxide-watch.service %{buildroot}%{_userunitdir}/oxide-watch.service

%post
%systemd_user_post oxide-watch.service

%preun
%systemd_user_preun oxide-watch.service

%files
%license LICENSE-MIT LICENSE-APACHE
%doc README.md CHANGELOG.md
%{_bindir}/oxide-embed
%{_userunitdir}/oxide-watch.service

%changelog
* Wed Sep 17 2026 Faez Barghasa <faez@oxide-tech.io> - 0.2.0-1
- Release 0.2.0: Multi-distro Linux installers, GraphRAG memory, and real Criterion benchmarks.

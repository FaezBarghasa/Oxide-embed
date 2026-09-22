use oxide_core::SymbolRecord;

pub struct OutlineGenerator;

impl OutlineGenerator {
    pub fn generate_outline(symbols: &[SymbolRecord]) -> String {
        let mut lines = Vec::new();
        for sym in symbols {
            let prefix = match sym.kind {
                oxide_core::SymbolKind::Function => "fn",
                oxide_core::SymbolKind::Method => "  fn",
                oxide_core::SymbolKind::Struct => "struct",
                oxide_core::SymbolKind::Union => "union",
                oxide_core::SymbolKind::Enum => "enum",
                oxide_core::SymbolKind::Trait => "trait",
                oxide_core::SymbolKind::Class => "class",
                oxide_core::SymbolKind::Interface => "interface",
                oxide_core::SymbolKind::TypeAlias => "type",
                oxide_core::SymbolKind::Constant => "const",
                oxide_core::SymbolKind::Static => "static",
                oxide_core::SymbolKind::Field => "field",
                oxide_core::SymbolKind::Module => "mod",
                oxide_core::SymbolKind::Import => "use",
                oxide_core::SymbolKind::Macro => "macro",
                oxide_core::SymbolKind::Other => "symbol",
            };

            let name = sym.qualified_name.as_deref().unwrap_or(&sym.name);
            if let Some(sig) = &sym.signature {
                lines.push(format!(
                    "{}: L{}-L{} | {}",
                    prefix, sym.start_line, sym.end_line, sig
                ));
            } else {
                lines.push(format!(
                    "{}: L{}-L{} | {}",
                    prefix, sym.start_line, sym.end_line, name
                ));
            }
        }
        lines.join("\n")
    }
}

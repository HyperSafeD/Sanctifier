use crate::rules::{Rule, RuleViolation, Severity};
use syn::{parse_str, Fields, File, Item, Meta, Type};

pub struct LedgerSizeRule {
    ledger_limit: usize,
    approaching_threshold: f64,
    strict_mode: bool,
}

impl LedgerSizeRule {
    pub fn new() -> Self {
        Self {
            ledger_limit: 64000,
            approaching_threshold: 0.8,
            strict_mode: false,
        }
    }
}

impl Default for LedgerSizeRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LedgerSizeRule {
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.ledger_limit = limit;
        self
    }

    pub fn with_approaching_threshold(mut self, threshold: f64) -> Self {
        self.approaching_threshold = threshold;
        self
    }

    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeWarningLevel {
    ExceedsLimit,
    ApproachingLimit,
}

impl Rule for LedgerSizeRule {
    fn name(&self) -> &str {
        "ledger_size"
    }

    fn description(&self) -> &str {
        "Analyzes contracttype structs and enums for ledger entry size limits"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut violations = Vec::new();
        let strict_threshold = (self.ledger_limit as f64 * 0.5) as usize;

        for item in &file.items {
            match item {
                Item::Struct(s) => {
                    if has_contracttype(&s.attrs) {
                        let size = self.estimate_struct_size(s);
                        if let Some(level) = self.classify_size(size, strict_threshold) {
                            let severity = match level {
                                SizeWarningLevel::ExceedsLimit => Severity::Error,
                                SizeWarningLevel::ApproachingLimit => Severity::Warning,
                            };
                            violations.push(RuleViolation::new(
                                self.name(),
                                severity,
                                format!("Struct '{}' estimated size {} bytes exceeds or approaches limit", s.ident, size),
                                format!("{}:estimated {} bytes, limit {} bytes", s.ident, size, self.ledger_limit),
                            ));
                        }
                    }
                }
                Item::Enum(e) => {
                    if has_contracttype(&e.attrs) {
                        let size = self.estimate_enum_size(e);
                        if let Some(level) = self.classify_size(size, strict_threshold) {
                            let severity = match level {
                                SizeWarningLevel::ExceedsLimit => Severity::Error,
                                SizeWarningLevel::ApproachingLimit => Severity::Warning,
                            };
                            violations.push(RuleViolation::new(
                                self.name(),
                                severity,
                                format!(
                                    "Enum '{}' estimated size {} bytes exceeds or approaches limit",
                                    e.ident, size
                                ),
                                format!(
                                    "{}:estimated {} bytes, limit {} bytes",
                                    e.ident, size, self.ledger_limit
                                ),
                            ));
                        }
                    }
                }
                _ => {}
            }
        }

        violations
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl LedgerSizeRule {
    fn classify_size(&self, size: usize, strict_threshold: usize) -> Option<SizeWarningLevel> {
        if size >= self.ledger_limit || (self.strict_mode && size >= strict_threshold) {
            Some(SizeWarningLevel::ExceedsLimit)
        } else if size as f64 >= self.ledger_limit as f64 * self.approaching_threshold {
            Some(SizeWarningLevel::ApproachingLimit)
        } else {
            None
        }
    }

    fn estimate_struct_size(&self, s: &syn::ItemStruct) -> usize {
        let mut total = 0;
        match &s.fields {
            Fields::Named(fields) => {
                for f in &fields.named {
                    total += self.estimate_type_size(&f.ty);
                }
            }
            Fields::Unnamed(fields) => {
                for f in &fields.unnamed {
                    total += self.estimate_type_size(&f.ty);
                }
            }
            Fields::Unit => {}
        }
        total
    }

    fn estimate_enum_size(&self, e: &syn::ItemEnum) -> usize {
        const DISCRIMINANT_SIZE: usize = 4;
        let mut max_variant = 0usize;
        for v in &e.variants {
            let mut variant_size = 0;
            match &v.fields {
                syn::Fields::Named(fields) => {
                    for f in &fields.named {
                        variant_size += self.estimate_type_size(&f.ty);
                    }
                }
                syn::Fields::Unnamed(fields) => {
                    for f in &fields.unnamed {
                        variant_size += self.estimate_type_size(&f.ty);
                    }
                }
                syn::Fields::Unit => {}
            }
            max_variant = max_variant.max(variant_size);
        }
        DISCRIMINANT_SIZE + max_variant
    }

    fn estimate_type_size(&self, ty: &Type) -> usize {
        match ty {
            Type::Path(tp) => {
                if let Some(seg) = tp.path.segments.last() {
                    let base = match seg.ident.to_string().as_str() {
                        "u32" | "i32" | "bool" => 4,
                        "u64" | "i64" => 8,
                        "u128" | "i128" | "I128" | "U128" => 16,
                        "Address" => 32,
                        "Bytes" | "BytesN" | "String" | "Symbol" => 64,
                        "Vec" => {
                            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                                    return 8 + self.estimate_type_size(inner);
                                }
                            }
                            128
                        }
                        "Map" => {
                            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                                let inner: usize = args
                                    .args
                                    .iter()
                                    .filter_map(|a| {
                                        if let syn::GenericArgument::Type(t) = a {
                                            Some(self.estimate_type_size(t))
                                        } else {
                                            None
                                        }
                                    })
                                    .sum();
                                if inner > 0 {
                                    return 16 + inner * 2;
                                }
                            }
                            128
                        }
                        "Option" => {
                            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                                    return 1 + self.estimate_type_size(inner);
                                }
                            }
                            32
                        }
                        _ => 32,
                    };
                    base
                } else {
                    8
                }
            }
            Type::Array(arr) => {
                if let syn::Expr::Lit(expr_lit) = &arr.len {
                    if let syn::Lit::Int(lit) = &expr_lit.lit {
                        if let Ok(n) = lit.base10_parse::<usize>() {
                            return n * self.estimate_type_size(&arr.elem);
                        }
                    }
                }
                64
            }
            _ => 8,
        }
    }
}

fn has_contracttype(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if let Meta::Path(path) = &attr.meta {
            path.is_ident("contracttype") || path.segments.iter().any(|s| s.ident == "contracttype")
        } else {
            false
        }
    })
}

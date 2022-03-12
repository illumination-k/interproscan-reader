use std::error::Error;

use crate::parser::{ast::ParseError, Expr};

pub const SOURCE_NAMES: [&str; 15] = [
    "MobiDBLite",
    "Gene3D",
    "ProSitePatterns",
    "PANTHER",
    "CDD",
    "Pfam",
    "SUPERFAMILY",
    "ProSiteProfiles",
    "PRINTS",
    "PIRSF",
    "TIGRFAM",
    "SMART",
    "Coils",
    "PIRSR",
    "SFLD",
];

pub fn validate_source_expr(source_expr: &Option<Expr>) -> Result<(), Box<dyn Error>> {
    if let Some(expr) = source_expr {
        if expr.matches(&SOURCE_NAMES)? {
            Ok(())
        } else {
            Err(Box::new(ParseError::new(format!(
                "Invalid source expr. Please select from [{}]",
                SOURCE_NAMES.join(" ")
            ))))
        }
    } else {
        Ok(())
    }
}

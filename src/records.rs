use crate::parser::Expr;
use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct DomainRecord {
    pub source: String,
    pub start: u64,
    pub end: u64,
    pub domain_name: String,
    pub domain_desc: String,
}

impl ToString for DomainRecord {
    fn to_string(&self) -> String {
        format!(
            "{}-{} {} {}",
            self.start, self.end, self.domain_name, self.domain_desc
        )
    }
}

impl DomainRecord {
    pub fn new<S: ToString>(
        source: S,
        start: u64,
        end: u64,
        domain_name: S,
        domain_desc: S,
    ) -> Self {
        Self {
            source: source.to_string(),
            start,
            end,
            domain_name: domain_name.to_string(),
            domain_desc: domain_desc.to_string(),
        }
    }

    pub fn is_gene(&self) -> bool {
        self.source == "."
    }
}

#[derive(Debug, Clone)]
pub struct GeneRecord {
    pub id: String,
    pub length: u64,
    domains: Vec<DomainRecord>,
}

impl GeneRecord {
    pub fn new(id: String, start: u64, end: u64) -> Self {
        Self {
            id,
            length: end - start + 1,
            domains: Vec::new(),
        }
    }

    pub fn push_domain(&mut self, domain: DomainRecord) {
        self.domains.push(domain);
    }

    pub fn iter_domains(&self) -> std::slice::Iter<'_, DomainRecord> {
        self.domains.iter()
    }

    pub fn filter_by_source_expr(self, source_expr: &Option<Expr>) -> Self {
        if let Some(expr) = source_expr {
            let domains: Vec<DomainRecord> = self
                .iter_domains()
                .filter(|domain| expr.matches(&[&domain.source]).expect("must ok"))
                .cloned()
                .collect();

            Self {
                id: self.id,
                length: self.length,
                domains,
            }
        } else {
            self
        }
    }

    pub fn to_tsv_line(&self) -> String {
        // gene_id source term_id term_desc start end
        let mut lines = Vec::with_capacity(self.domains.len() + 1);
        lines.push(format!("{}\t.\t.\t.\t0\t{}", self.id, self.length));

        for domain in self.domains.iter() {
            lines.push(format!(
                "{}\t{}\t{}\t{}\t{}\t{}",
                self.id,
                domain.source,
                domain.domain_name,
                domain.domain_desc,
                domain.start,
                domain.end,
            ));
        }

        lines.join("\n")
    }

    pub fn to_table_row(&self) -> Vec<Vec<String>> {
        let mut cells = Vec::with_capacity(self.domains.len() + 1);

        cells.push(vec![
            self.id.to_owned(),
            ".".to_string(),
            ".".to_string(),
            ".".to_string(),
            "0".to_string(),
            self.length.to_string(),
        ]);

        for domain in self.domains.iter() {
            cells.push(vec![
                self.id.to_owned(),
                domain.source.to_owned(),
                domain.domain_name.to_owned(),
                domain.domain_desc.to_owned(),
                domain.start.to_string(),
                domain.end.to_string(),
            ])
        }

        cells
    }
}

impl Display for GeneRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let header = format!("--- id: {}, length {} ---", self.id, self.length);
        let domains = self
            .domains
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
            .join("\n");

        write!(f, "{}\n{}", header, domains)
    }
}

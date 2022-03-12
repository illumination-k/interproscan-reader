use std::{
    collections::HashMap,
    error::Error,
    ffi::OsStr,
    fmt::Display,
    fs::File,
    io::{self, BufRead, BufReader},
    path::Path,
};

use flate2::read::MultiGzDecoder;

use crate::parser::Expr;

fn is_compressed<P: AsRef<Path>>(p: &P) -> bool {
    let ext = p.as_ref().extension();

    ext == Some(OsStr::new("gz"))
}

pub fn read_with_gz<P: AsRef<Path>>(p: &P) -> Result<Box<dyn BufRead>, Box<dyn Error>> {
    let file = File::open(p)?;
    let reader: Box<dyn BufRead> = if is_compressed(p) {
        let gz = MultiGzDecoder::new(file);
        Box::new(BufReader::new(gz))
    } else {
        Box::new(BufReader::new(file))
    };

    Ok(reader)
}

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

pub fn parse_line(line: &str) -> Result<(String, DomainRecord), Box<dyn Error>> {
    let line = line.trim();

    let records: Vec<&str> = line.split('\t').collect();
    if records.len() != 9 {
        let err: Box<dyn Error> = Box::new(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid line: {}", line),
        ));

        return Err(err);
    }

    let id = records[0];
    let source = records[1];
    let start: u64 = records[3].parse()?;
    let end: u64 = records[4].parse()?;

    let mut domain_name = "No Name";
    let mut domain_desc = "No Description";
    for attr in records[8].split(';') {
        let attr_records: Vec<&str> = attr.split('=').collect();

        if attr_records.len() != 2 {
            continue;
        }

        if attr_records[0] == "Name" {
            domain_name = attr_records[1];
        } else if attr_records[0] == "signature_desc" {
            domain_desc = attr_records[1]
        }
    }

    Ok((
        id.to_string(),
        DomainRecord::new(source, start, end, domain_name, domain_desc),
    ))
}

#[derive(Debug, Clone)]
pub struct GeneRecord {
    pub id: String,
    length: u64,
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

    pub fn to_tsv_record(&self) -> String {
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

#[must_use]
pub struct InterproGffReader<R: BufRead> {
    reader: R,
    comment: char,
    finish_line: String,
    id_expr: Option<Expr>,
    domain_expr: Option<Expr>,
    source_expr: Option<Expr>,
    max_length: Option<u64>,
    min_length: Option<u64>,
}

impl<R: BufRead> InterproGffReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            comment: '#',
            finish_line: "## FASTA ##".to_string(),
            id_expr: None,
            domain_expr: None,
            source_expr: None,
            max_length: None,
            min_length: None,
        }
    }

    pub fn with_comment(mut self, comment: char) -> Self {
        self.comment = comment;
        self
    }

    #[allow(dead_code)]
    pub fn with_finish_line(mut self, finish_line: String) -> Self {
        self.finish_line = finish_line;
        self
    }

    pub fn with_id_expr(mut self, expr: Option<Expr>) -> Self {
        self.id_expr = expr;
        self
    }

    pub fn with_domain_expr(mut self, expr: Option<Expr>) -> Self {
        self.domain_expr = expr;
        self
    }

    pub fn with_source_expr(mut self, expr: Option<Expr>) -> Self {
        self.source_expr = expr;
        self
    }

    pub fn with_max_length(mut self, length: Option<u64>) -> Self {
        self.max_length = length;
        self
    }

    pub fn with_min_length(mut self, length: Option<u64>) -> Self {
        self.min_length = length;
        self
    }

    pub fn finish(self) -> Result<Vec<GeneRecord>, Box<dyn Error>> {
        let mut records_map = HashMap::new();

        for line in self.reader.lines() {
            let line = line?;
            if line.starts_with(&self.finish_line) {
                break;
            }

            if line.starts_with(self.comment) {
                continue;
            }

            if line.len() == 1 {
                continue;
            }

            let (id, domain) = parse_line(&line)?;

            if let Some(expr) = &self.id_expr {
                if !expr.matches(&[&id])? {
                    continue;
                }
            }

            if domain.is_gene() {
                let gene_record = GeneRecord::new(id.clone(), domain.start, domain.end);

                if let Some(max_length) = self.max_length {
                    if gene_record.length > max_length {
                        continue;
                    }
                }

                if let Some(min_length) = self.min_length {
                    if gene_record.length < min_length {
                        continue;
                    }
                }

                records_map.entry(id).or_insert(gene_record);
            } else {
                if let Some(gene_record) = records_map.get_mut(&id) {
                    gene_record.push_domain(domain);
                }
            }
        }

        let records = records_map
            .into_values()
            .filter(|x| {
                if let Some(expr) = &self.domain_expr {
                    let expr_result = expr.matches_domains(x);

                    if let Ok(is_ok) = expr_result {
                        is_ok
                    } else {
                        false
                    }
                } else {
                    true
                }
            })
            .map(|d| d.filter_by_source_expr(&self.source_expr))
            .collect();

        Ok(records)
    }
}

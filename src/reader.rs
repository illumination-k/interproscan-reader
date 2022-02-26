use std::{
    collections::HashMap,
    error::Error,
    ffi::OsStr,
    io::{self, BufRead, Read},
    path::Path,
};

pub fn is_compressed<P: AsRef<Path>>(p: &P) -> bool {
    let ext = p.as_ref().extension();

    if ext == Some(OsStr::new("gz")) {
        true
    } else {
        false
    }
}

#[derive(Debug, Clone)]
pub struct DomainRecord {
    pub source: String,
    pub start: u64,
    pub end: u64,
    pub domain_name: String,
    pub domain_desc: String,
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

    let records: Vec<&str> = line.split("\t").collect();
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
    for attr in records[8].split(";") {
        let attr_records: Vec<&str> = attr.split("=").collect();

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
    id: String,
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
}

#[must_use]
pub struct InterproGffReader<R: BufRead> {
    reader: R,
    comment: char,
    finish_line: String,
}

impl<R: BufRead> InterproGffReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            comment: '#',
            finish_line: "## FASTA ##".to_string(),
        }
    }

    pub fn with_comment(mut self, comment: char) -> Self {
        self.comment = comment;
        self
    }

    pub fn with_finish_line(mut self, finish_line: String) -> Self {
        self.finish_line = finish_line;
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

            if domain.is_gene() {
                records_map.entry(id.clone()).or_insert(GeneRecord::new(
                    id,
                    domain.start,
                    domain.end,
                ));
            } else {
                if let Some(gene_record) = records_map.get_mut(&id) {
                    gene_record.push_domain(domain);
                }
            }
        }

        let records = records_map.into_values().collect();

        Ok(records)
    }
}

use std::io::{self, BufReader};
use std::str::FromStr;
use std::{str, env};

use indexmap::IndexMap;
use main_error::MainError;
use quick_xml::events::Event;
use quick_xml::Reader;

type Scope = Vec<Vec<u8>>;

#[derive(Clone, PartialEq, Eq)]
struct XPath {
    path: Vec<Vec<u8>>,
    attribute: Option<Vec<u8>>,
}

impl FromStr for XPath {
    type Err = ();
    fn from_str(s: &str) -> Result<XPath, Self::Err> {
        let (path, attribute) = match s.find('@') {
            Some(p) => {
                let (left, right) = s.split_at(p);
                let attribute = right[1..].as_bytes().to_vec();
                (left, Some(attribute))
            },
            None => (s, None),
        };

        let path = path.split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.as_bytes().to_vec())
            .collect();

        Ok(XPath { path, attribute })
    }
}

// impl Debug for XPath

fn main() -> Result<(), MainError> {
    let xml = BufReader::new(io::stdin());
    let mut reader = Reader::from_reader(xml);
    reader.trim_text(true);

    let mut headers_xpaths = IndexMap::new();
    for arg in env::args().skip(1) {
        let mut iter = arg.splitn(2, '=');
        let header = iter.next().unwrap().trim().to_owned();
        let xpath = iter.next().unwrap().trim().to_owned();
        if xpath.is_empty() { panic!("xpath ({}) must be defined", header) }
        let xpath = XPath::from_str(&xpath).unwrap();
        headers_xpaths.insert(header, xpath);
    }

    let mut writer = csv::Writer::from_writer(io::stdout());
    writer.write_record(headers_xpaths.keys())?;

    let mut buf = Vec::new();
    let mut scope = Scope::default();
    let shortest_scope = headers_xpaths.values().min_by_key(|p| p.path.len()).unwrap();
    // We reserve the entry strings, that we will clean.
    let mut completeness = vec![false; headers_xpaths.len()];
    let mut entry = vec![String::new(); headers_xpaths.len()];

    loop {
        match reader.read_event(&mut buf)? {
            Event::Start(e) => {
                scope.push(e.name().to_owned());

                for (i, xpath) in headers_xpaths.values().enumerate() {
                    if let Some(attribute) = &xpath.attribute {
                        if &scope == &xpath.path {
                            if let Some(Ok(attr)) = e.attributes().find(|a| a.as_ref().map_or(false, |a| a.key == &attribute[..])) {
                                let text = str::from_utf8(&attr.value).unwrap();
                                entry[i].clear();
                                entry[i].push_str(text);
                                completeness[i] = true;
                            }
                        }
                    }
                }
            },
            Event::End(e) => {
                if &scope == &shortest_scope.path {
                    completeness.iter_mut().for_each(|x| *x = false);
                }

                // TODO not sure that is the best solution!
                for xpath in headers_xpaths.values() {
                    if &scope == &xpath.path {
                        if completeness.iter().all(|c| *c) {
                            writer.write_record(&entry)?;
                            break;
                        }
                    }
                }

                // only pop if we entered in a scope
                if scope.last().as_ref().map(AsRef::as_ref) == Some(e.name()) {
                    scope.pop();
                }
            },

            Event::Text(e) => {
                for (i, xpath) in headers_xpaths.values().enumerate() {
                    if xpath.attribute.is_none() && &scope == &xpath.path {
                        let unescaped = e.unescaped()?;
                        let text = reader.decode(&unescaped)?;

                        entry[i].clear();
                        entry[i].push_str(text);
                        completeness[i] = true;
                    }
                }
            },

            Event::Eof => break,
            _ => (),
        }
        buf.clear();
    }

    writer.flush()?;

    Ok(())
}

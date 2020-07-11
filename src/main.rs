use std::io::{self, BufReader};
use std::mem;

use quick_xml::Reader;
use quick_xml::events::Event;
use smallstr::SmallString;
use main_error::MainError;

type SmallString64 = SmallString<[u8; 64]>;
type Scope = Vec<Vec<u8>>;

#[derive(Default)]
struct Document {
    title: Option<SmallString64>,
    abstrac: Option<String>,
    url: Option<String>,
}

fn main() -> Result<(), MainError> {
    let xml = BufReader::new(io::stdin());
    let mut reader = Reader::from_reader(xml);
    reader.trim_text(true);

    let mut writer = csv::Writer::from_writer(io::stdout());
    writer.write_record(&["title", "abstract", "url"])?;

    let mut count = 0;
    let mut buf = Vec::new();
    let mut scope = Scope::default();
    let mut document = Document::default();

    loop {
        match reader.read_event(&mut buf)? {
            Event::Start(e) => {
                scope.push(e.name().to_owned());
            },
            Event::End(e) => {
                // only pop if we entered in a scope
                if scope.last().as_ref().map(AsRef::as_ref) == Some(e.name()) {
                    scope.pop();
                }

                match e.name() {
                    b"doc" => {
                        // end of document, we must write the csv line if complete
                        if let Document { title: Some(title), abstrac: Some(abstrac), url: Some(url), } = mem::take(&mut document) {
                            writer.write_record(&[
                                title.as_str(),
                                abstrac.as_str(),
                                url.as_str()])?;
                        }
                    },
                    _ => (),
                }
            },

            Event::Text(e) => {
                let unescaped = e.unescaped()?;
                let text = reader.decode(&unescaped)?;

                if scope == [&b"feed"[..], b"doc", b"title"] {
                    document.title = Some(SmallString64::from_str(text));
                }

                if scope == [&b"feed"[..], b"doc", b"url"] {
                    document.url = Some(text.to_string());
                }

                if scope == [&b"feed"[..], b"doc", b"abstract"] {
                    count += 1;
                    if count % 100000 == 0 { eprintln!("{}k documents seen", count / 1000) }
                    document.abstrac = Some(text.to_string());
                }
            },

            Event::Eof => break,
            _ => (),
        }
        buf.clear();
    }

    eprintln!("{} documents seen", count);

    writer.flush()?;

    Ok(())
}
